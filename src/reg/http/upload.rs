use std::io::Read;
use std::ops::DerefMut;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::fs::File;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Client;
use reqwest::Method;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio::task::JoinHandle;

use crate::progress::{CoreStatus, ProcessResult, Processor, ProcessorAsync, ProgressStatus};
use crate::reg::http::{do_request_raw_read, HttpAuth};
use crate::reg::BlobConfig;

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
    Finished { file_size: u64, finished_reason: String },
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
                file_size,
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

impl Processor<UploadResult> for RegUploader {
    fn start(&self) -> Box<dyn ProcessorAsync<UploadResult>> {
        return match &self.reg_uploader_enum {
            RegUploaderEnum::Finished {
                file_size: _,
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
                let handle = tokio::spawn(async move {
                    let uploader = reg_http_uploader;
                    let result = uploading(status.clone(), file_path_clone.clone().as_str(), uploader, blob_config_arc).await;
                    let status_core = &mut status.status_core.lock().unwrap();
                    status_core.done = true;
                    if let Err(err) = &result {
                        Err(anyhow!("{}", err))
                    } else {
                        Ok(UploadResult {
                            result_str: "succuss".to_string(),
                        })
                    }
                });
                Box::new(RegUploadHandler { join: handle })
            }
        };
    }

    fn process_status(&self) -> Box<dyn ProgressStatus> {
        Box::new(self.temp.clone())
    }
}

async fn uploading(
    status: RegUploaderStatus,
    file_path: &str,
    reg_http_uploader: RegHttpUploader,
    blob_config: Arc<BlobConfig>,
) -> Result<String> {
    //检查本地是否存在已有
    let file_path = Path::new(file_path);
    let local_file = File::open(file_path).await?;
    let file_size = local_file.metadata().await?.len();
    let reader = RegUploaderReader {
        status,
        file: local_file,
    };
    let response = do_request_raw_read::<RegUploaderReader>(
        &reg_http_uploader.client,
        reg_http_uploader.url.as_str(),
        Method::PUT,
        Some(&reg_http_uploader.auth),
        &[],
        Some(reader),
        file_size,
    )
        .await?;
    let short_hash = &blob_config.short_hash;
    if response.status().is_success() {
        let body = response.text().await.unwrap_or("".to_string());
        Ok(body)
    } else {
        let status_code = response.status();
        let body = response.text().await.unwrap_or("".to_string());
        Err(anyhow!(
            "{} upload request failed. status_code: {} body: {}",
            short_hash,
            status_code,
            body
        ))
    }
}

pub struct RegUploaderReader {
    status: RegUploaderStatus,
    file: File,
}

impl AsyncRead for RegUploaderReader {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        let read = self.file.poll_read(cx, buf)
            .map_ok(|| {
                let mut guard = self.status.status_core.lock().unwrap();
                let core = guard.deref_mut();
                core.curr_size += size as u64;
            });
        read
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

impl ProgressStatus for RegUploaderStatus {
    fn status(&self) -> CoreStatus {
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

impl ProcessResult for UploadResult {
    fn finished_info(&self) -> &str {
        &self.result_str
    }
}
