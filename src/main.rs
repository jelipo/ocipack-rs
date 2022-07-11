#![feature(exclusive_range_pattern)]
#![feature(once_cell)]

extern crate derive_builder;

use std::ops::Deref;
use std::sync::{Arc, LazyLock};

use anyhow::Result;
use clap::Parser;
use home::home_dir;

use crate::config::cmd::CmdArgs;
use crate::config::global::GlobalAppConfig;
use crate::reg::home::HomeDir;
use crate::reg::CompressType;
use crate::subcmd::build::BuildCommand;
use crate::subcmd::clean::CleanCommand;
use crate::subcmd::transform::TransformCommand;

mod adapter;
mod bar;
mod config;
mod const_data;
mod init;
mod progress;
mod reg;
mod subcmd;
mod util;

/// 全局共享的Config
pub static GLOBAL_CONFIG: LazyLock<GlobalAppConfig> = LazyLock::new(init_config);

pub static CACHE_DIR_NAME: &str = "pack_cache";

fn main() -> Result<()> {
    init::init()?;
    let global_config = GLOBAL_CONFIG.deref();
    match &global_config.cmd_args {
        CmdArgs::Build(build_args) => {
            BuildCommand::build(build_args)?;
        }
        CmdArgs::Transform(transform_args) => {
            TransformCommand::transform(transform_args)?;
        }
        CmdArgs::Clean(clean_args) => {
            CleanCommand::clean(clean_args)?;
        }
    }
    Ok(())
}

fn init_config() -> GlobalAppConfig {
    let home_path = home_dir().expect("can not found home dir");
    let cache_dir = home_path.join(CACHE_DIR_NAME);
    GlobalAppConfig {
        cmd_args: CmdArgs::parse(),
        home_dir: Arc::new(HomeDir::new_home_dir(&cache_dir).expect("")),
    }
}
