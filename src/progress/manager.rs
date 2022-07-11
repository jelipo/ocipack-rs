use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;

use crate::bar::{Bar, MultiBar};
use crate::progress::{ProcessResult, Processor, ProcessorAsync, ProgressStatus};

pub struct ProcessorManager<R: ProcessResult> {
    statuses: Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>, Bar)>,
    multi_progress: MultiBar,
}

impl<R: ProcessResult> ProcessorManager<R> {
    pub fn new_processor_manager(processors: Vec<Box<dyn Processor<R>>>) -> Result<ProcessorManager<R>> {
        let mut mb = MultiBar::new_multi_bar();
        let status = processors
            .iter()
            .map(|processor| {
                let async_processor = processor.start();
                let status = processor.process_status();
                let status_core = status.status();
                let name = status_core.blob_config.short_hash.clone();
                let bar = mb.add_new_bar(name, status_core.full_size);
                (async_processor, status, bar)
            })
            .collect::<Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>, Bar)>>();
        Ok(ProcessorManager {
            statuses: status,
            multi_progress: mb,
        })
    }

    pub fn size(&self) -> usize {
        self.statuses.len()
    }

    pub fn wait_all_done(mut self) -> Result<Vec<R>> {
        println!();
        let mut statuses = self.statuses;
        let mut result_infos = Vec::<R>::new();
        while !statuses.is_empty() {
            let mut new_status: Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>, Bar)> = Vec::new();
            for (processor, progress_status, mut bar) in statuses {
                let status = &progress_status.status();
                bar.set_size(status.now_size, status.full_size);
                if status.is_done {
                    let process_result = processor.wait_result()?;
                    let finished_info = process_result.finished_info();
                    bar.finish(true, finished_info);
                    result_infos.push(process_result);
                } else {
                    new_status.push((processor, progress_status, bar))
                }
            }
            self.multi_progress.update();
            if new_status.is_empty() {
                break;
            }
            statuses = new_status;
            sleep(Duration::from_secs(1));
        }
        println!();
        Ok(result_infos)
    }
}
