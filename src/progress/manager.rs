use crate::progress::Processor;
use anyhow::{Error, Result};

pub struct ProcessorManager<R> {
    processors: Vec<Box<dyn Processor<R>>>,
}

impl<R> ProcessorManager<R> {
    pub fn new_processor_manager() -> Result<ProcessorManager<R>> {
        Err(Error::msg(""))
    }
}