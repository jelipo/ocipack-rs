use std::collections::VecDeque;
use std::time::Duration;

use anyhow::Result;
use tokio::task::JoinSet;
use tokio::time::MissedTickBehavior;

use crate::bar::{Bar, MultiBar};
use crate::progress::{ProcessResult, ProgressStatusEnum, TransferJob};

const MAX_CONCURRENT_TRANSFERS: usize = 4;

pub struct ProcessorManager<R: ProcessResult + Send + 'static> {
    jobs: Vec<TransferJob<R>>,
}

impl<R: ProcessResult + Send + 'static> ProcessorManager<R> {
    pub fn new_processor_manager(jobs: Vec<TransferJob<R>>) -> Self {
        Self { jobs }
    }

    pub fn size(&self) -> usize {
        self.jobs.len()
    }

    pub async fn wait_all_done(self) -> Result<Vec<R>> {
        let mut multi_progress = MultiBar::new_multi_bar();
        let mut statuses: Vec<(ProgressStatusEnum, Bar)> = Vec::with_capacity(self.jobs.len());
        let mut pending = VecDeque::with_capacity(self.jobs.len());
        for (index, job) in self.jobs.into_iter().enumerate() {
            let core = job.status.status();
            let bar = multi_progress.add_new_bar(core.blob_config.short_hash.clone(), core.full_size);
            statuses.push((job.status, bar));
            pending.push_back((index, job.future));
        }

        let mut active = JoinSet::new();
        let total = statuses.len();
        let mut results: Vec<Option<R>> = (0..total).map(|_| None).collect();
        let mut completed = 0;
        let mut refresh = tokio::time::interval(Duration::from_millis(200));
        refresh.set_missed_tick_behavior(MissedTickBehavior::Skip);
        println!();

        while completed < total {
            while active.len() < MAX_CONCURRENT_TRANSFERS {
                let Some((index, future)) = pending.pop_front() else { break };
                active.spawn(async move { (index, future.await) });
            }

            tokio::select! {
                Some(joined) = active.join_next(), if !active.is_empty() => {
                    let (index, result) = match joined {
                        Ok(output) => output,
                        Err(error) => {
                            active.shutdown().await;
                            return Err(error.into());
                        }
                    };
                    match result {
                        Ok(result) => {
                            let core = statuses[index].0.status();
                            statuses[index].1.set_size(core.now_size, core.full_size);
                            statuses[index].1.finish(true, result.finished_info());
                            results[index] = Some(result);
                            completed += 1;
                        }
                        Err(error) => {
                            statuses[index].1.finish(false, &error.to_string());
                            multi_progress.update();
                            active.shutdown().await;
                            return Err(error);
                        }
                    }
                }
                _ = refresh.tick() => {
                    for (status, bar) in &mut statuses {
                        let core = status.status();
                        bar.set_size(core.now_size, core.full_size);
                    }
                    multi_progress.update();
                }
            }
        }
        multi_progress.update();
        println!();
        Ok(results.into_iter().map(Option::unwrap).collect())
    }
}
