use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::Result;
use reqwest::blocking::{Client, Response};
use reqwest::Method;

use crate::Processor;
use crate::progress::{CoreStatus, ProcessorAsync, ProgressStatus};
use crate::reg::BlobConfig;
use crate::reg::docker::http::HttpAuth;

pub struct RegUploader {
    reg_uploader_enum: RegUploaderEnum,
    blob_config: Arc<BlobConfig>,
}

pub struct RegFinishedUploader {
    result: String,
}

impl ProcessorAsync<String> for RegFinishedUploader {
    fn wait_result(self: Box<Self>) -> Result<String> {
        Ok(self.result)
    }
}

struct RegUploaderCore {
    url: String,
    auth: HttpAuth,
    client: Client,
    temp: RegUploaderStatus,
    blob_config: Arc<BlobConfig>,
}

enum RegUploaderEnum {
    Finished { blob_config: Arc<BlobConfig>, file_size: usize },
    Run(RegUploaderCore),
}

#[derive(Clone)]
pub struct RegUploaderStatus {
    status_core: Arc<Mutex<RegUploaderStatusCore>>,
}

struct RegUploaderStatusCore {
    blob_config: Arc<BlobConfig>,
    file_size: usize,
    pub curr_size: usize,
    pub done: bool,
}

impl RegUploader {
    /// 创建一个已经完成状态的Uploader
    pub fn new_finished_uploader(blob_config: BlobConfig, file_size: usize) -> RegUploader {
        let blob_config_arc = Arc::new(blob_config);
        RegUploader {
            reg_uploader_enum: RegUploaderEnum::Finished {
                blob_config: blob_config_arc.clone(),
                file_size,
            },
            blob_config: blob_config_arc,
        }
    }

    pub fn new_uploader(url: String, auth: HttpAuth, client: Client, blob_config: BlobConfig) -> RegUploader {
        let blob_config_arc = Arc::new(blob_config);
        let temp = RegUploaderStatus {
            status_core: Arc::new(Mutex::new(RegUploaderStatusCore {
                blob_config: blob_config_arc.clone(),
                file_size: 0,
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
                    temp,
                    blob_config: blob_config_arc.clone(),
                }
            },
            blob_config: blob_config_arc,
        }
    }
}

impl Processor<String> for RegUploader {
    fn start(&self) -> Box<dyn ProcessorAsync<String>> {
        return match &self.reg_uploader_enum {
            RegUploaderEnum::Finished { blob_config, file_size } => Box::new(RegFinishedUploader {
                result: "already exists".to_string()
            }),
            RegUploaderEnum::Run(info) => {
                let status = info.temp.clone();
                let reg_http_uploader = RegHttpUploader {
                    url: info.url.clone(),
                    auth: info.auth.clone(),
                    client: info.client.clone(),
                };
                let file_path_clone = self.blob_config.file_path.to_str().unwrap().to_string();
                let handle = thread::spawn::<_, Result<String>>(move || {
                    let uploader = reg_http_uploader;
                    let result = downloading(status.clone(), file_path_clone.clone().as_str(), downloader);
                    let status_core = &mut status.status_core.lock().unwrap();
                    status_core.done = true;
                    if let Err(err) = &result {
                        println!("{}\n{}", err, err.backtrace());
                    }
                    Ok(file_path_clone)
                });
                Box::new(RegDownloadHandler {
                    join: handle
                })
            }
        };
    }

    fn process_status(&self) -> Box<dyn ProgressStatus> {
        match &self.reg_uploader_enum {
            RegUploaderEnum::Finished { blob_config, file_size } => {
                Box::new(info.temp.clone())
            }
            RegUploaderEnum::Run(info) => {
                Box::new(info.temp.clone())
            }
        }
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

impl RegHttpUploader {
    fn do_request_raw(&self) -> Result<Response> {
        let url = self.url.as_str();
        do_request_raw::<u8>(&self.client, url, Method::GET, &self.auth, &None, None)
    }
}