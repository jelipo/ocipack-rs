use std::sync::Arc;

use anyhow::Result;

use crate::container::http::download::{RegDownloadHandler, RegDownloader, RegDownloaderStatus, RegFinishedDownloader};
use crate::container::http::upload::{RegFinishedUploader, RegUploadHandler, RegUploader, RegUploaderStatus};
use crate::container::BlobConfig;

pub mod manager;

pub enum ProcessorAsyncEnum {
    RegDownloadHandler(RegDownloadHandler),
    RegFinishedDownloader(RegFinishedDownloader),
    RegFinishedUploader(RegFinishedUploader),
    RegUploadHandler(RegUploadHandler),
}

pub enum ProcessorEnum {
    RegDownloader(RegDownloader),
    RegUploader(RegUploader),
}

impl ProcessorEnum {
    pub async fn start(self) -> ProcessorAsyncEnum {
        match self {
            ProcessorEnum::RegDownloader(d) => d.start().await,
            ProcessorEnum::RegUploader(d) => d.start().await,
        }
    }

    pub fn process_status(&self) -> ProgressStatusEnum {
        match self {
            ProcessorEnum::RegDownloader(downloader) => downloader.process_status(),
            ProcessorEnum::RegUploader(uploader) => uploader.process_status(),
        }
    }
}

pub trait Processor<R> {
    async fn start(&self) -> ProcessorAsyncEnum;

    fn process_status(&self) -> ProgressStatusEnum;
}

pub trait ProcessorAsync<R> {
    async fn wait_result(self: Box<Self>) -> Result<R>;
}

pub struct CoreStatus {
    pub blob_config: Arc<BlobConfig>,
    pub full_size: u64,
    pub now_size: u64,
    pub is_done: bool,
}

pub enum ProgressStatusEnum {
    RegDownloaderStatus(RegDownloaderStatus),
    RegUploaderStatus(RegUploaderStatus),
}

pub trait ProgressStatus {
    fn status(&self) -> CoreStatus;
}

pub trait ProcessResult {
    fn finished_info(&self) -> &str;
}
