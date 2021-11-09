
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
            let mut done_vec = vec![false; self.statuses.len()];
            for (index, (_, progress_status)) in self.statuses.iter().enumerate() {
                let status = progress_status.status();
                done_vec[index] = status.is_done;
                println!("{} 进度,下载了 {} KiB", status.name.as_str(), status.now_size / 1024);
            }
            if processors_all_done(&done_vec[..]) {
                break;
            }
            sleep(Duration::from_secs(1))
        }
        Ok(())
    }
}

fn processors_all_done(done_vec: &[bool]) -> bool {
    for is_done in done_vec {
        if !is_done { return false; }
    }
    true
}