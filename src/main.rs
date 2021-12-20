#![feature(exclusive_range_pattern)]
#![feature(once_cell)]


use std::borrow::Borrow;
use std::lazy::SyncLazy;

use anyhow::Result;
use clap::Parser;

use crate::config::cmd::CmdArgs;

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