use std::env;
use std::lazy::{OnceCell, SyncLazy};
use std::sync::Arc;
use clap::Parser;

use anyhow::Result;
use env_logger::Env;

use crate::{CmdArgs, GlobalAppConfig};


pub fn init() -> Result<()> {
    log_init();

    Ok(())
}

fn log_init() {
    let env = Env::default()
        .default_filter_or("info");
    env_logger::init_from_env(env);
}
