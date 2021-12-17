#![feature(exclusive_range_pattern)]
#![feature(once_cell)]

use std::lazy::OnceCell;
use clap::Parser;
use anyhow::Result;
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


fn main() -> Result<()> {
    let args: CmdArgs = CmdArgs::parse();

    docker::run()
}