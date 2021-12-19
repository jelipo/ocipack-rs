#![feature(exclusive_range_pattern)]
#![feature(once_cell)]

use std::lazy::{OnceCell, SyncLazy};

use anyhow::Result;
use clap::Parser;
use env_logger::Env;
use log::Level::Info;

use crate::config::cmd::CmdArgs;
use crate::config::global::GlobalAppConfig;

mod progress;
mod reg;
mod util;
mod bar;
mod config;
mod docker;
mod tempconfig;
mod adapter;
mod init;

pub static GLOBAL_CONFIG: SyncLazy<CmdArgs> = SyncLazy::new(|| {
    let args: CmdArgs = CmdArgs::parse();
    args
});

fn main() -> Result<()> {
    init::init()?;
    docker::run()
}