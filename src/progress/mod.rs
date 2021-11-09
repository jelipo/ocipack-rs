use std::sync::{Arc, Mutex};
use anyhow::Result;

pub mod manager;

pub trait Processor<R> {
    fn start(&self) -> Box<dyn ProcessorAsync<R>>;

    fn process_status(&self) -> Box<dyn ProgressStatus>;
}

pub trait ProcessorAsync<R> {
    fn wait_result(self: Box<Self>) -> Result<R>;
}

pub struct CoreStatus {
    pub name: Arc<String>,
    pub full_size: usize,
    pub now_size: usize,
    pub is_done: bool,
}

pub trait ProgressStatus {
    fn status(&self) -> CoreStatus;
}