use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use reqwest::Method;
use tokio::fs::File;
use tokio::io::{AsyncRead, ReadBuf};
use tokio::task::JoinHandle;
use crate::container::BlobConfig;
use crate::container::http::{do_request_raw_read, HttpAuth};
use crate::progress::{CoreStatus, Processor, ProcessorAsync, ProcessResult, ProgressStatus};

pub struct RegUploader {
    reg_uploader_enum: RegUploaderEnum,
    blob_config: Arc<BlobConfig>,
    temp: RegUploaderStatus,
}

pub struct RegFinishedUploader {
    upload_result: UploadResult,
}

#[async_trait]
impl ProcessorAsync<UploadResult> for RegFinishedUploader {
    async fn wait_result(self: Box<Self>) -> Result<UploadResult> {
        Ok(self.upload_result)
    }
}

struct RegUploaderCore {
    url: String,
    auth: HttpAuth,
    client: Client,
}

enum RegUploaderEnum {
    Finished { _file_size: u64, finished_reason: String },
    Run(RegUploaderCore),
}

#[derive(Clone)]
pub struct RegUploaderStatus {
    status_core: Arc<Mutex<RegUploaderStatusCore>>,
}

struct RegUploaderStatusCore {
    blob_config: Arc<BlobConfig>,
    file_size: u64,
    pub curr_size: u64,
    pub done: bool,
}

impl RegUploader {
    /// 创建一个已经完成状态的Uploader
    pub fn new_finished_uploader(blob_config: BlobConfig, file_size: u64, finished_reason: String) -> RegUploader {
        let blob_config_arc = Arc::new(blob_config);
        let temp = RegUploaderStatus {
            status_core: Arc::new(Mutex::new(RegUploaderStatusCore {
                blob_config: blob_config_arc.clone(),
                file_size,
                curr_size: file_size,
                done: true,
            })),
        };
        RegUploader {
            reg_uploader_enum: RegUploaderEnum::Finished {
                _file_size: file_size,
                finished_reason,
            },
            blob_config: blob_config_arc,
            temp,
        }
    }

    pub fn new_uploader(url: String, auth: HttpAuth, client: Client, blob_config: BlobConfig, file_size: u64) -> RegUploader {
        let blob_config_arc = Arc::new(blob_config);
        let temp = RegUploaderStatus {
            status_core: Arc::new(Mutex::new(RegUploaderStatusCore {
                blob_config: blob_config_arc.clone(),
                file_size,
                curr_size: 0,
                done: false,
            })),
        };
        RegUploader {
            reg_uploader_enum: RegUploaderEnum::Run(RegUploaderCore { url, auth, client }),
            blob_config: blob_config_arc,
            temp,
        }
    }
}

#[async_trait]
impl Processor<UploadResult> for RegUploader {
    async fn start(&self) -> Box<dyn ProcessorAsync<UploadResult>> {
        return match &self.reg_uploader_enum {
            RegUploaderEnum::Finished {
                _file_size: _,
                finished_reason,
            } => Box::new(RegFinishedUploader {
                upload_result: UploadResult {
                    result_str: finished_reason.to_string(),
                },
            }),
            RegUploaderEnum::Run(info) => {
                let status = self.temp.clone();
                let reg_http_uploader = RegHttpUploader {
                    url: info.url.clone(),
                    auth: info.auth.clone(),
                    client: info.client.clone(),
                };
                let file_path_clone = self.blob_config.file_path.to_str().unwrap().to_string();
                let blob_config_arc = self.blob_config.clone();
                let handle = tokio::spawn(upload(reg_http_uploader, status, file_path_clone, blob_config_arc));
                Box::new(RegUploadHandler { join: handle })
            }
        };
    }

    async fn process_status(&self) -> Box<dyn ProgressStatus> {
        Box::new(self.temp.clone())
    }
}

async fn upload(reg_http_uploader: RegHttpUploader, status: RegUploaderStatus, file_path_clone: String, blob_config_arc: Arc<BlobConfig>) -> Result<UploadResult> {
    let uploader = reg_http_uploader;
    let result = uploading(status.clone(), file_path_clone.clone().as_str(), uploader, blob_config_arc).await;
    let status_core = &mut status.status_core.lock().unwrap();
    status_core.done = true;
    if let Err(err) = &result {
        Err(anyhow!("{}\n{}", err, err.backtrace()))
    } else {
        Ok(UploadResult {
            result_str: "succuss".to_string(),
        })
    }
}

async fn uploading(status: RegUploaderStatus, file_path: &str, reg_http_uploader: RegHttpUploader, blob_config: Arc<BlobConfig>) -> Result<()> {
    //检查本地是否存在已有
    let file_path = Path::new(file_path);
    let local_file = File::open(file_path).await?;
    let file_size = local_file.metadata().await?.len();
    let reader = RegUploaderReader { status, file: local_file };
    let response = do_request_raw_read::<RegUploaderReader>(
        &reg_http_uploader.client,
        reg_http_uploader.url.as_str(),
        Method::PUT,
        Some(&reg_http_uploader.auth),
        &[],
        Some(reader),
        file_size,
    ).await?;
    let short_hash = &blob_config.short_hash;
    if !response.status().is_success() {
        let status_code = response.status().to_string();
        let response_string = response.text().await.unwrap_or_default();
        Err(anyhow!("{} upload request failed. status_code: {} body: {}",status_code, short_hash, response_string))
    } else {
        Ok(())
    }
}

pub struct RegUploaderReader {
    status: RegUploaderStatus,
    file: File,
}

impl AsyncRead for RegUploaderReader {
    fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.file).poll_read(cx, buf)
    }
}

pub struct RegUploadHandler {
    join: JoinHandle<Result<UploadResult>>,
}

#[async_trait]
impl ProcessorAsync<UploadResult> for RegUploadHandler {
    async fn wait_result(self: Box<Self>) -> Result<UploadResult> {
        self.join.await.map_err(|_| anyhow!("join failed."))?
    }
}

#[async_trait]
impl ProgressStatus for RegUploaderStatus {
    async fn status(&self) -> CoreStatus {
        let core = &self.status_core.lock().unwrap();
        CoreStatus {
            blob_config: core.blob_config.clone(),
            full_size: core.file_size,
            now_size: core.curr_size,
            is_done: core.done,
        }
    }
}

struct RegHttpUploader {
    url: String,
    auth: HttpAuth,
    client: Client,
}

pub struct UploadResult {
    pub result_str: String,
}

#[async_trait]
impl ProcessResult for UploadResult {
    async fn finished_info(&self) -> &str {
        &self.result_str
    }
}
