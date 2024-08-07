use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use futures::future::join_all;

use crate::bar::{Bar, MultiBar};
use crate::progress::{Processor, ProcessorAsync, ProcessResult, ProgressStatus};

pub struct ProcessorManager<R: ProcessResult> {
    statuses: Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>, Bar)>,
    multi_progress: MultiBar,
}

async fn process<R: ProcessResult>(processor: &Box<dyn Processor<R>>, mut mb: MultiBar) -> (Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>, Bar) {
    let async_processor = processor.start().await;
    let status = processor.process_status().await;
    let status_core = status.status().await;
    let name = status_core.blob_config.short_hash.clone();
    let bar = mb.add_new_bar(name, status_core.full_size);
    (async_processor, status, bar)
}

impl<R: ProcessResult> ProcessorManager<R> {
    pub async fn new_processor_manager(processors: Vec<Box<dyn Processor<R>>>) -> Result<ProcessorManager<R>> {
        let mb = MultiBar::new_multi_bar();
        let futures = processors.iter().map(|processor| process(processor, mb.clone())).collect::<Vec<_>>();
        let status = join_all(futures).await;
        Ok(ProcessorManager {
            statuses: status,
            multi_progress: mb,
        })
    }

    pub fn size(&self) -> usize {
        self.statuses.len()
    }

    pub async fn wait_all_done(mut self) -> Result<Vec<R>> {
        println!();
        let mut statuses = self.statuses;
        let mut result_infos = Vec::<R>::new();
        while !statuses.is_empty() {
            let mut new_status: Vec<(Box<dyn ProcessorAsync<R>>, Box<dyn ProgressStatus>, Bar)> = Vec::new();
            for (processor, progress_status, mut bar) in statuses {
                let status = &progress_status.status().await;
                bar.set_size(status.now_size, status.full_size);
                if status.is_done {
                    let process_result = processor.wait_result().await?;
                    let finished_info = process_result.finished_info().await;
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
