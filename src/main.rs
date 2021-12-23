#![feature(exclusive_range_pattern)]
#![feature(once_cell)]

#[macro_use]
extern crate derive_builder;

use std::borrow::Borrow;
use std::lazy::SyncLazy;

use anyhow::Result;
use clap::Parser;

use crate::config::cmd::CmdArgs;
use crate::subcmd::build::BuildCommand;

mod progress;
mod reg;
mod util;
mod bar;
mod config;
mod docker;
mod tempconfig;
mod adapter;
mod init;
mod subcmd;
mod pull;

pub static GLOBAL_CONFIG: SyncLazy<CmdArgs> = SyncLazy::new(|| {
    let args: CmdArgs = CmdArgs::parse();
    args
});


fn main() -> Result<()> {
    init::init()?;
    let cmd_args: &CmdArgs = GLOBAL_CONFIG.borrow();
    match cmd_args {
        CmdArgs::Build(build_args) => {
            let command = BuildCommand::build(build_args)?;
        }
        CmdArgs::Transform => {}
    }
    docker::run()
}