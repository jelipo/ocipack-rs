use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use crate::progress::{Processor, ProcessorAsync, ProgressStatus};
use anyhow::{Error, Result};

pub struct ProcessorManager<R> {
    status: Vec<(Box<dyn ProcessorAsync<R>>, Arc<Mutex<dyn ProgressStatus>>)>,
}

impl<R> ProcessorManager<R> {
    pub fn new_processor_manager(processors: Vec<Box<dyn Processor<R>>>) -> Result<ProcessorManager<R>> {
        let status = processors.iter().map(|processor| {
            let async_processor = processor.start();
            let status = processor.process_status();
            (async_processor, status)
        }).collect::<Vec<(Box<dyn ProcessorAsync<R>>, Arc<Mutex<dyn ProgressStatus>>)>>();
        Ok(ProcessorManager {
            status
        })
    }

    pub fn wait_all_done(self) -> Result<()> {
        loop {
            for (async_processor, statuses) in &self.status {
                let status = statuses.lock().unwrap();
                println!("{} 进度,下载了 {} 字节", status.name(), status.now_size() / 1024)
            }
            sleep(Duration::from_secs(1))
        }
        Ok(())
    }
}