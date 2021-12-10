use std::borrow::BorrowMut;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use anyhow::{Error, Result};
use log::info;
use reqwest::blocking::Client;
use reqwest::Method;

use crate::Processor;
use crate::progress::{CoreStatus, ProcessorAsync, ProcessResult, ProgressStatus};
use crate::reg::BlobConfig;
use crate::reg::docker::http::{do_request_raw_read, HttpAuth};

pub struct RegUploader {
    reg_uploader_enum: RegUploaderEnum,
    blob_config: Arc<BlobConfig>,
    temp: RegUploaderStatus,
}

pub struct RegFinishedUploader {
    upload_result: UploadResult,
}

impl ProcessorAsync<UploadResult> for RegFinishedUploader {
    fn wait_result(self: Box<Self>) -> Result<UploadResult> {
        Ok(self.upload_result)
    }
}

struct RegUploaderCore {
    url: String,
    auth: HttpAuth,
    client: Client,
    blob_config: Arc<BlobConfig>,
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
            }))
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
            }))
        };
        RegUploader {
            reg_uploader_enum: RegUploaderEnum::Run {
                0: RegUploaderCore {
                    url,
                    auth,
                    client,
                    blob_config: blob_config_arc.clone(),
                }
            },
            blob_config: blob_config_arc,
            temp,
        }
    }
}

impl Processor<UploadResult> for RegUploader {
    fn start(&self) -> Box<dyn ProcessorAsync<UploadResult>> {
        return match &self.reg_uploader_enum {
            RegUploaderEnum::Finished { file_size: _, finished_reason } => Box::new(RegFinishedUploader {
                upload_result: UploadResult {
                    result_str: finished_reason.to_string()
                }
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
                let handle = thread::spawn::<_, Result<UploadResult>>(move || {
                    let uploader = reg_http_uploader;
                    let result = uploading(status.clone(), file_path_clone.clone().as_str(), uploader, blob_config_arc);
                    let status_core = &mut status.status_core.lock().unwrap();
                    status_core.done = true;
                    if let Err(err) = &result {
                        println!("{}\n{}", err, err.backtrace());
                    }
                    Ok(UploadResult {
                        result_str: "succuss".to_string()
                    })
                });
                Box::new(RegUploadHandler {
                    join: handle
                })
            }
        };
    }

    fn process_status(&self) -> Box<dyn ProgressStatus> {
        Box::new(self.temp.clone())
    }
}

fn uploading(
    status: RegUploaderStatus, file_path: &str, reg_http_uploader: RegHttpUploader, blob_config: Arc<BlobConfig>,
) -> Result<()> {
    //检查本地是否存在已有
    let file_path = Path::new(file_path);
    let local_file = File::open(file_path)?;
    let file_size = local_file.metadata()?.len();
    let reader = RegUploaderReader {
        status,
        file: local_file,
    };
    let mut response = do_request_raw_read::<RegUploaderReader>(
        &reg_http_uploader.client, &reg_http_uploader.url.as_str(), Method::PUT,
        Some(&reg_http_uploader.auth), None, Some(reader), file_size)?;
    let short_hash = &blob_config.short_hash;
    if response.status().is_success() {
        let mut response_string = String::new();
        let _read_size = response.read_to_string(&mut response_string)?;
        Ok(())
    } else {
        let _status_code = response.status().as_str();
        let mut response_string = String::new();
        let _read_size = response.read_to_string(&mut response_string)?;
        Err(Error::msg(format!("{} upload request failed. {}", short_hash, response_string)))
    }
}

pub struct RegUploaderReader {
    status: RegUploaderStatus,
    file: File,
}

impl Read for RegUploaderReader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let size = self.file.read(buf)?;
        let mut guard = self.status.status_core.lock().unwrap();
        let core = guard.borrow_mut();
        core.curr_size = core.curr_size + size as u64;
        Ok(size)
    }
}

pub struct RegUploadHandler {
    join: JoinHandle<Result<UploadResult>>,
}

impl ProcessorAsync<UploadResult> for RegUploadHandler {
    fn wait_result(self: Box<Self>) -> Result<UploadResult> {
        let result = self.join.join();
        result.unwrap()
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