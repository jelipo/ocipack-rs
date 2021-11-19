use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Error, Result};
use indicatif::{MultiProgress, ProgressBar};

use crate::progress::{Processor, ProcessorAsync, ProgressStatus};

pub struct ProcessorManager<R> {
    statuses: Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>, ProgressBar)>,
    multi_progress: MultiProgress,
}

impl<R> ProcessorManager<R> {
    pub fn new_processor_manager(processors: Vec<Box<dyn Processor<R>>>) -> Result<ProcessorManager<R>> {
        let multi_progress = MultiProgress::new();
        let status = processors.iter().map(|processor| {
            let async_processor = processor.start();
            let status = processor.process_status();
            let progress_bar = multi_progress.add(ProgressBar::new(status.status().full_size as u64));
            (async_processor, status, progress_bar)
        }).collect::<Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>, ProgressBar)>>();
        Ok(ProcessorManager {
            statuses: status,
            multi_progress,
        })
    }

    pub fn wait_all_done(self) -> Result<()> {
        println!("开始等待");
        let multi_progress = self.multi_progress;
        std::thread::spawn(move || {
            println!("进度条开始");
            multi_progress.join();
            println!("进度条完成");
        });
        loop {
            let mut done_vec = vec![false; self.statuses.len()];
            for (index, (_, progress_status, bar)) in self.statuses.iter().enumerate() {
                let status = progress_status.status();
                done_vec[index] = status.is_done;
                bar.inc(status.now_size as u64);
                if status.is_done {
                    if !bar.is_finished() {
                        bar.finish()
                    }
                }
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