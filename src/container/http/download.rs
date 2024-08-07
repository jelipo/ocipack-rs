use std::borrow::BorrowMut;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

use anyhow::{anyhow, Result};
use reqwest::blocking::{Client, Response};
use reqwest::Method;

use crate::container::http::{do_request_raw, get_header, HttpAuth};
use crate::container::BlobConfig;
use crate::progress::{CoreStatus, ProcessResult, Processor, ProcessorAsync, ProgressStatus};

pub struct RegDownloader {
    finished: bool,
    url: String,
    auth: Option<HttpAuth>,
    client: Option<Client>,
    temp: RegDownloaderStatus,
    blob_down_config: Arc<BlobConfig>,
}

impl RegDownloader {
    pub fn new_reg(
        url: String,
        auth: Option<HttpAuth>,
        client: Client,
        blob_down_config: BlobConfig,
        layer_size: Option<u64>,
    ) -> Result<RegDownloader> {
        let blob_down_config_arc = Arc::new(blob_down_config);
        let temp = RegDownloaderStatus {
            status_core: Arc::new(Mutex::new(RegDownloaderStatusCore {
                blob_config: blob_down_config_arc.clone(),
                file_size: layer_size.unwrap_or(0),
                curr_size: 0,
                done: false,
            })),
        };
        Ok(RegDownloader {
            finished: false,
            url,
            auth,
            client: Some(client),
            temp,
            blob_down_config: blob_down_config_arc,
        })
    }

    pub fn new_finished(blob_down_config: BlobConfig, file_size: u64) -> Result<RegDownloader> {
        let blob_down_config_arc = Arc::new(blob_down_config);
        let temp = RegDownloaderStatus {
            status_core: Arc::new(Mutex::new(RegDownloaderStatusCore {
                blob_config: blob_down_config_arc.clone(),
                file_size,
                curr_size: 0,
                done: true,
            })),
        };
        Ok(RegDownloader {
            finished: true,
            url: String::default(),
            auth: None,
            client: None,
            temp,
            blob_down_config: blob_down_config_arc,
        })
    }
}

impl Processor<DownloadResult> for RegDownloader {
    fn start(&self) -> Box<dyn ProcessorAsync<DownloadResult>> {
        let blob_config = self.blob_down_config.clone();
        let file_path = blob_config.file_path.clone();
        let status = self.temp.clone();
        if self.finished {
            return Box::new(RegFinishedDownloader {
                result: DownloadResult {
                    file_path: Some(file_path.clone()),
                    _file_size: file_path.metadata().unwrap().len(),
                    blob_config,
                    local_existed: true,
                    result_str: "local exists".to_string(),
                },
            });
        }
        let reg_http_downloader = RegHttpDownloader {
            url: self.url.clone(),
            auth: self.auth.clone(),
            client: self.client.as_ref().unwrap().clone(),
        };
        let handle = thread::spawn::<_, Result<DownloadResult>>(move || {
            let downloader = reg_http_downloader;
            let result = downloading(status.clone(), &file_path, downloader);
            let status_core = &mut status.status_core.lock().unwrap();
            status_core.done = true;
            if let Err(err) = &result {
                println!("{}\n{}", err, err.backtrace());
            }
            Ok(DownloadResult {
                file_path: Some(file_path),
                _file_size: status_core.file_size,
                blob_config: blob_config.clone(),
                local_existed: false,
                result_str: "complete".to_string(),
            })
        });
        Box::new(RegDownloadHandler { join: handle })
    }

    fn process_status(&self) -> Box<dyn ProgressStatus> {
        Box::new(self.temp.clone())
    }
}

pub struct RegDownloadHandler {
    join: JoinHandle<Result<DownloadResult>>,
}

impl ProcessorAsync<DownloadResult> for RegDownloadHandler {
    fn wait_result(self: Box<Self>) -> Result<DownloadResult> {
        let result = self.join.join();
        result.unwrap()
    }
}

pub struct RegFinishedDownloader {
    result: DownloadResult,
}

impl ProcessorAsync<DownloadResult> for RegFinishedDownloader {
    fn wait_result(self: Box<Self>) -> Result<DownloadResult> {
        Ok(self.result)
    }
}

fn downloading(status: RegDownloaderStatus, file_path: &Path, reg_http_downloader: RegHttpDownloader) -> Result<()> {
    //检查本地是否存在已有
    let parent_path = file_path.parent().expect("find file parent dir failed");
    if !parent_path.exists() {
        let _create_result = std::fs::create_dir(parent_path);
    }
    // 请求HTTP下载
    let mut http_response = reg_http_downloader.do_request_raw()?;
    check(&http_response)?;
    if let Some(len) = http_response.content_length() {
        let mut status_core = status.status_core.lock().expect("lock failed");
        status_core.borrow_mut().file_size = len;
    }
    let file = File::create(file_path)?;
    let mut writer = RegDownloaderWriter { status, file };
    let _copy_size = std::io::copy(&mut http_response, &mut writer)?;
    writer.flush()?;
    Ok(())
}

struct RegHttpDownloader {
    url: String,
    auth: Option<HttpAuth>,
    client: Client,
}

impl RegHttpDownloader {
    fn do_request_raw(&self) -> Result<Response> {
        let url = self.url.as_str();
        do_request_raw::<u8>(&self.client, url, Method::GET, self.auth.as_ref(), &[], None, None)
    }
}

const OCTET_STREAM_TYPE: [&str; 2] = [
    "binary/octet-stream", // quay.io registry use this type
    "application/octet-stream",
];

fn check(response: &Response) -> Result<()> {
    let headers = response.headers();
    let content_type = get_header(headers, "content-type").ok_or_else(|| anyhow!("content-type not found"))?;
    if !OCTET_STREAM_TYPE.contains(&content_type.as_str()) {
        return Err(anyhow!("Not support the content type:{}", content_type));
    }
    Ok(())
}

pub struct RegDownloaderWriter {
    status: RegDownloaderStatus,
    file: File,
}

impl Write for RegDownloaderWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let status_core = &mut self.status.status_core.lock().unwrap();
        status_core.curr_size += buf.len() as u64;
        self.file.write_all(buf)?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

#[derive(Clone)]
pub struct RegDownloaderStatus {
    status_core: Arc<Mutex<RegDownloaderStatusCore>>,
}

struct RegDownloaderStatusCore {
    blob_config: Arc<BlobConfig>,
    file_size: u64,
    pub curr_size: u64,
    pub done: bool,
}

impl ProgressStatus for RegDownloaderStatus {
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

pub struct DownloadResult {
    pub file_path: Option<Box<Path>>,
    pub _file_size: u64,
    pub blob_config: Arc<BlobConfig>,
    pub local_existed: bool,
    pub result_str: String,
}

impl ProcessResult for DownloadResult {
    fn finished_info(&self) -> &str {
        &self.result_str
    }
}
