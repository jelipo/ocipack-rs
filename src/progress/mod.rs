use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Result;

use crate::container::BlobConfig;
use crate::container::http::download::RegDownloaderStatus;
use crate::container::http::upload::RegUploaderStatus;

pub mod manager;

pub struct TransferJob<R> {
    pub status: ProgressStatusEnum,
    pub future: Pin<Box<dyn Future<Output = Result<R>> + Send>>,
}

pub struct CoreStatus {
    pub blob_config: Arc<BlobConfig>,
    pub full_size: u64,
    pub now_size: u64,
}

pub enum ProgressStatusEnum {
    RegDownloaderStatus(RegDownloaderStatus),
    RegUploaderStatus(RegUploaderStatus),
}

impl ProgressStatusEnum {
    pub fn status(&self) -> CoreStatus {
        match self {
            Self::RegDownloaderStatus(status) => status.status(),
            Self::RegUploaderStatus(status) => status.status(),
        }
    }
}

pub trait ProgressStatus {
    fn status(&self) -> CoreStatus;
}

pub trait ProcessResult {
    fn finished_info(&self) -> &str;
}
