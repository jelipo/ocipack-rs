use std::sync::{Arc, Mutex};
use anyhow::Result;

pub mod manager;

pub trait Processor<R> {
    fn start(&self) -> Box<dyn ProcessorAsync<R>>;

    fn process_status(&self) -> Arc<Mutex<dyn ProgressStatus>>;
}

pub trait ProcessorAsync<R> {
    fn wait_result(self: Box<Self>) -> Result<R>;
}

pub trait ProgressStatus {
    fn full_size(&self) -> usize;

    fn now_size(&self) -> usize;

    fn is_done(&self) -> bool;
}