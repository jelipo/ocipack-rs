use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use crate::container::BlobConfig;

pub mod manager;

#[async_trait]
pub trait Processor<R> {
    async fn start(&self) -> Box<dyn ProcessorAsync<R> >;

    async fn process_status(&self) -> Box<dyn ProgressStatus>;
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

#[async_trait]
pub trait ProgressStatus {
    async fn status(&self) -> CoreStatus;
}

#[async_trait]
pub trait ProcessResult {
    async fn finished_info(&self) -> &str;
}
