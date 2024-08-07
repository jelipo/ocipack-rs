use std::borrow::BorrowMut;
use std::io::Error;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::TryStreamExt;
use reqwest::{Client, Response};
use reqwest::Method;
use tokio::fs::File;
use tokio::io;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::container::BlobConfig;
use crate::container::http::{do_request_raw, get_header, HttpAuth};
use crate::progress::{CoreStatus, Processor, ProcessorAsync, ProcessResult, ProgressStatus};

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

#[async_trait]
impl Processor<DownloadResult> for RegDownloader {
    async fn start(&self) -> Box<dyn ProcessorAsync<DownloadResult>> {
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
        let handle = tokio::spawn(download_result(status, reg_http_downloader, file_path, blob_config.clone()));
        Box::new(RegDownloadHandler { join: handle })
    }

    async fn process_status(&self) -> Box<dyn ProgressStatus> {
        Box::new(self.temp.clone())
    }
}

async fn download_result(status: RegDownloaderStatus, reg_http_downloader: RegHttpDownloader, file_path: Box<Path>, blob_config: Arc<BlobConfig>) -> Result<DownloadResult> {
    let downloader = reg_http_downloader;
    let result = downloading(status.clone(), &file_path, downloader).await;
    let status_core = &mut status.status_core.lock().await;
    status_core.done = true;
    if let Err(err) = &result {
        println!("{}\n{}", err, err.backtrace());
    }
    Ok(DownloadResult {
        file_path: Some(file_path),
        _file_size: status_core.file_size,
        blob_config,
        local_existed: false,
        result_str: "complete".to_string(),
    })
}

pub struct RegDownloadHandler {
    join: JoinHandle<Result<DownloadResult>>,
}

#[async_trait]
impl ProcessorAsync<DownloadResult> for RegDownloadHandler {
    async fn wait_result(self: Box<Self>) -> Result<DownloadResult> {
        let result = self.join.await;
        result.unwrap()
    }
}

pub struct RegFinishedDownloader {
    result: DownloadResult,
}

#[async_trait]
impl ProcessorAsync<DownloadResult> for RegFinishedDownloader {
    async fn wait_result(self: Box<Self>) -> Result<DownloadResult> {
        Ok(self.result)
    }
}

async fn downloading(status: RegDownloaderStatus, file_path: &Path, reg_http_downloader: RegHttpDownloader) -> Result<()> {
    //检查本地是否存在已有
    let parent_path = file_path.parent().expect("find file parent dir failed");
    if !parent_path.exists() {
        let _create_result = std::fs::create_dir(parent_path);
    }
    // 请求HTTP下载
    let http_response = reg_http_downloader.do_request_raw().await?;

    check(&http_response)?;
    if let Some(len) = http_response.content_length() {
        let mut status_core = status.status_core.lock().await;
        status_core.borrow_mut().file_size = len;
    }
    let file = File::create(file_path).await?;
    let mut writer = RegDownloaderWriter { status, file };

    let stream = http_response.bytes_stream()
        .map_err(|e| Error::new(io::ErrorKind::Other, e));
    let mut read = stream.into_async_read().compat();
    let _copy_size = tokio::io::copy(&mut read, &mut writer).await?;
    writer.flush().await?;
    Ok(())
}

struct RegHttpDownloader {
    url: String,
    auth: Option<HttpAuth>,
    client: Client,
}

impl RegHttpDownloader {
    async fn do_request_raw(&self) -> Result<Response> {
        let url = self.url.as_str();
        do_request_raw::<u8>(&self.client, url, Method::GET, self.auth.as_ref(), &[], None, None).await
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

impl AsyncWrite for RegDownloaderWriter {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::result::Result<usize, Error>> {
        let mut pinned_file = Pin::new(&mut self.file);
        match pinned_file.poll_write(cx, buf) {
            Poll::Ready(Ok(size)) => {
                let status_clone = Arc::clone(&self.status);
                let fut = async move {
                    let mut status = status_clone.lock().await;
                    status.bytes_written += size;
                };
                tokio::spawn(fut);
                Poll::Ready(Ok(size))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
            Poll::Pending => Poll::Pending,
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Error>> {
        Pin::new(&self.file).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::result::Result<(), Error>> {
        Pin::new(&self.file).poll_shutdown(cx)
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

#[async_trait]
impl ProgressStatus for RegDownloaderStatus {
    async fn status(&self) -> CoreStatus {
        let core = &self.status_core.lock().await;
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

#[async_trait]
impl ProcessResult for DownloadResult {
    async fn finished_info(&self) -> &str {
        &self.result_str
    }
}
