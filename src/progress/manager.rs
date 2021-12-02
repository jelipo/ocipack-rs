use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;

use crate::bar::{Bar, MultiBar};
use crate::progress::{Processor, ProcessorAsync, ProgressStatus};

pub struct ProcessorManager<R> {
    statuses: Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>, Bar)>,
    multi_progress: MultiBar,
}

impl<R> ProcessorManager<R> {
    pub fn new_processor_manager(processors: Vec<Box<dyn Processor<R>>>) -> Result<ProcessorManager<R>> {
        let mut mb = MultiBar::new_multi_bar();
        let status = processors.iter().map(|processor| {
            let async_processor = processor.start();
            let status = processor.process_status();
            let status_core = status.status();
            let name = status_core.blob_config.short_hash.clone();
            let bar = mb.add_new_bar(name, status_core.full_size);
            (async_processor, status, bar)
        }).collect::<Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>, Bar)>>();
        Ok(ProcessorManager {
            statuses: status,
            multi_progress: mb,
        })
    }

    pub fn wait_all_done(self) -> Result<()> {
        println!("开始等待");
        let mut statuses = self.statuses;
        loop {
            let mut done_vec = vec![false; statuses.len()];
            for index in 0..statuses.len() {
                let (processor, progress_status, bar) = &mut statuses[index];
                let r = processor.wait_result()?;
                let status = progress_status.status();
                done_vec[index] = status.is_done;
                bar.add_size(status.now_size);
                if status.is_done {
                    bar.finish(true);
                }
            }
            if processors_all_done(&done_vec[..]) {
                break;
            }
            sleep(Duration::from_secs(1));
            self.multi_progress.update();
        }
        for index in 0..statuses.len() {
            let (processor, _, _) = &mut statuses[index];
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