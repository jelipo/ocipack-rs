use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use crate::progress::{Processor, ProcessorAsync, ProgressStatus};
use anyhow::{Error, Result};

pub struct ProcessorManager<R> {
    statuses: Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>)>,
}

impl<R> ProcessorManager<R> {
    pub fn new_processor_manager(processors: Vec<Box<dyn Processor<R>>>) -> Result<ProcessorManager<R>> {
        let status = processors.iter().map(|processor| {
            let async_processor = processor.start();
            let status = processor.process_status();
            (async_processor, status)
        }).collect::<Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>)>>();
        Ok(ProcessorManager {
            statuses: status
        })
    }

    pub fn wait_all_done(self) -> Result<()> {
        println!("开始等待");
        loop {
            for (async_processor, status) in &self.statuses {
                println!("{} 进度,下载了 {} KiB", status.name(), status.now_size() / 1024)
            }
            if processors_all_done(&self.statuses) {
                break;
            }
            sleep(Duration::from_secs(1))
        }
        Ok(())
    }
}

fn processors_all_done<R>(status: &Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>)>) -> bool {
    for (_, status) in status {
        if !status.is_done() {
            return false;
        }
    }
    true
}