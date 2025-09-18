use std::sync::Arc;

use anyhow::Result;

use crate::container::BlobConfig;
use crate::container::http::download::{RegDownloadHandler, RegFinishedDownloader};
use crate::container::http::upload::{RegFinishedUploader, RegUploadHandler};

pub mod manager;

pub enum ProcessorAsyncEnum {
    RegDownloadHandler(RegDownloadHandler),
    RegFinishedDownloader(RegFinishedDownloader),
    RegFinishedUploader(RegFinishedUploader),
    RegUploadHandler(RegUploadHandler),
}

pub trait Processor<R> {
    async fn start(&self) -> ProcessorAsyncEnum;

    fn process_status(&self) -> Box<dyn ProgressStatus>;
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

pub trait ProgressStatus {
    fn status(&self) -> CoreStatus;
}

pub trait ProcessResult {
    fn finished_info(&self) -> &str;
}
