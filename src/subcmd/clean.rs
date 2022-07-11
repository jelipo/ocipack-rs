use crate::config::cmd::CleanCmdArgs;
use crate::{GLOBAL_CONFIG, GlobalAppConfig};

pub struct CleanCommand {}

impl CleanCommand {
    pub fn clean(clean_args: &CleanCmdArgs) -> anyhow::Result<()> {
        let home_dir = GLOBAL_CONFIG.home_dir.clone();
        // TODO
        Ok(())
    }
}