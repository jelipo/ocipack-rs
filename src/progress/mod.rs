use std::sync::Arc;

use anyhow::Result;

use crate::reg::BlobConfig;

pub mod manager;

pub trait Processor<R> {
    fn start(&self) -> Box<dyn ProcessorAsync<R>>;

    fn process_status(&self) -> Box<dyn ProgressStatus>;
}

pub trait ProcessorAsync<R> {
    fn wait_result(self: Box<Self>) -> Result<R>;
}

pub struct CoreStatus {
    pub blob_config: Arc<BlobConfig>,
    pub full_size: usize,
    pub now_size: usize,
    pub is_done: bool,
}

pub trait ProgressStatus {
    fn status(&self) -> CoreStatus;
}