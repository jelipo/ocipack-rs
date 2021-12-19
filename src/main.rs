#![feature(exclusive_range_pattern)]
#![feature(once_cell)]

use std::any::Any;
use std::borrow::Borrow;
use std::lazy::{OnceCell, SyncLazy, SyncOnceCell};
use std::sync::Arc;

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
    let x: &CmdArgs = GLOBAL_CONFIG.borrow();
    match x {
        CmdArgs::Build(a) => {
            println!("{}", a.allow_insecure);
        }
        CmdArgs::Transform => {}
    }
    init::init()?;
    docker::run()
}