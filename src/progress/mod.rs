use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::reg::BlobConfig;

pub mod manager;

#[async_trait]
pub trait Processor<R> {
    fn start(&self) -> Box<dyn ProcessorAsync<R>>;

    fn process_status(&self) -> Box<dyn ProgressStatus>;
}

#[async_trait]
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
