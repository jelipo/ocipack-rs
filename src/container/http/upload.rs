use crate::container::BlobConfig;
use crate::container::http::{HttpAuth, do_request_raw_read};
use crate::progress::{CoreStatus, ProcessResult, Processor, ProcessorAsync, ProcessorAsyncEnum, ProgressStatus};
use anyhow::{Result, anyhow};
use reqwest::Client;
use reqwest::Method;
use std::io::read_to_string;
use std::ops::DerefMut;
use std::path::Path;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio::task::JoinHandle;
use tokio_util::io::InspectReader;

pub struct RegUploader {
    reg_uploader_enum: RegUploaderEnum,
    blob_config: Arc<BlobConfig>,
    temp: RegUploaderStatus,
}

pub struct RegFinishedUploader {
    upload_result: UploadResult,
}

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

impl Processor<UploadResult> for RegUploader {
    async fn start(&self) -> ProcessorAsyncEnum {
        return match &self.reg_uploader_enum {
            RegUploaderEnum::Finished {
                _file_size: _,
                finished_reason,
            } => ProcessorAsyncEnum::RegFinishedUploader(RegFinishedUploader {
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
                        Err(anyhow!("{}\n{}", err, err.backtrace()))
                    } else {
                        Ok(UploadResult {
                            result_str: "succuss".to_string(),
                        })
                    }
                });
                ProcessorAsyncEnum::RegUploadHandler(RegUploadHandler { join: handle })
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
) -> Result<()> {
    //检查本地是否存在已有
    let file_path = Path::new(file_path);
    let local_file = File::open(file_path).await?;
    let file_size = local_file.metadata().await?.len();
    let reader = RegUploaderReader::new(status, local_file);
    let mut response = do_request_raw_read::<RegUploaderReader>(
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
        let _body = response.text().await?;
        Ok(())
    } else {
        let status_code = response.status().as_u16();
        let response_string = response.text().await?;
        Err(anyhow!(
            "{} upload request failed. code: {}, body: {}",
            short_hash,
            status_code,
            response_string
        ))
    }
}

pub struct RegUploaderReader {
    status: RegUploaderStatus,
    inspect_reader: InspectReader<File, Box<dyn Fn(&[u8]) + Send + Sync>>,
}

impl AsyncRead for RegUploaderReader {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.get_mut().inspect_reader).poll_read(cx, buf)
    }
}

impl RegUploaderReader {
    fn new(status: RegUploaderStatus, file: File) -> Self {
        let status_clone = status.clone();
        let inspect_reader = InspectReader::new(
            file,
            Box::new(move |bytes: &[u8]| {
                if let Ok(mut guard) = status_clone.status_core.lock() {
                    guard.curr_size += bytes.len() as u64;
                }
            }),
        );

        RegUploaderReader {
            status,
            inspect_reader: inspect_reader,
        }
    }
}

pub struct RegUploadHandler {
    join: JoinHandle<Result<UploadResult>>,
}

impl ProcessorAsync<UploadResult> for RegUploadHandler {
    async fn wait_result(mut self: Box<Self>) -> Result<UploadResult> {
        self.join.await?
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
