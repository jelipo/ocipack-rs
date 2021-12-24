#![feature(exclusive_range_pattern)]
#![feature(once_cell)]

#[macro_use]
extern crate derive_builder;

use std::borrow::Borrow;
use std::lazy::SyncLazy;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use home::home_dir;

use crate::config::cmd::CmdArgs;
use crate::config::global::GlobalAppConfig;
use crate::reg::home::HomeDir;
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
mod push;

pub static GLOBAL_CONFIG: SyncLazy<GlobalAppConfig> = SyncLazy::new(|| {
    let home_path = home_dir().expect("can not found home dir");
    GlobalAppConfig {
        cmd_args: CmdArgs::parse(),
        home_dir: Arc::new(HomeDir::new_home_dir(&home_path).expect("")),
    }
});


fn main() -> Result<()> {
    init::init()?;
    let global_config: &GlobalAppConfig = GLOBAL_CONFIG.borrow();
    match &global_config.cmd_args {
        CmdArgs::Build(build_args) => {
            let command = BuildCommand::build(build_args)?;
        }
        CmdArgs::Transform => {}
    }
    docker::run()
}