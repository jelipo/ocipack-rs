#![feature(exclusive_range_pattern)]
#![feature(once_cell)]
#![feature(async_iter_from_iter)]
#![feature(async_closure)]

extern crate derive_builder;

use std::ops::Deref;
use std::sync::{Arc, LazyLock};

use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use home::home_dir;

use crate::config::cmd::CmdArgs;
use crate::config::global::GlobalAppConfig;
use crate::reg::home::HomeDir;
use crate::reg::CompressType;
use crate::subcmd::build::BuildCommand;
use crate::subcmd::clean::CleanCommand;
use crate::subcmd::show_info::ShowInfoCommand;
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

#[tokio::main]
async fn main() -> Result<()> {
    init::init()?;
    let global_config = GLOBAL_CONFIG.deref();
    print_log();
    match &global_config.cmd_args {
        CmdArgs::Build(build_args) => {
            BuildCommand::build(build_args).await?;
        }
        CmdArgs::Transform(transform_args) => {
            TransformCommand::transform(transform_args).await?;
        }
        CmdArgs::Clean(clean_args) => {
            CleanCommand::clean(clean_args)?;
        }
        CmdArgs::ShowInfo(show_info_args) => {
            ShowInfoCommand::show(show_info_args).await?;
        }
    }
    Ok(())
}

fn init_config() -> GlobalAppConfig {
    let home_path = home_dir().expect("can not found home dir");
    let cache_dir = home_path.join(CACHE_DIR_NAME);
    GlobalAppConfig {
        cmd_args: CmdArgs::parse(),
        home_dir: Arc::new(HomeDir::new_home_dir(&cache_dir).expect("home dir build")),
    }
}

fn print_log() {
    let logo_1_1 = r#" ________  ________  ___   "#.green();
    let logo_1_2 = r#"|\   __  \|\   ____\|\  \  "#.green();
    let logo_1_3 = r#"\ \  \|\  \ \  \___|\ \  \ "#.green();
    let logo_1_4 = r#" \ \  \\\  \ \  \    \ \  \ "#.green();
    let logo_1_5 = r#"  \ \  \\\  \ \  \____\ \  \ "#.green();
    let logo_1_6 = r#"   \ \_______\ \_______\ \__\"#.green();
    let logo_1_7 = r#"    \|_______|\|_______|\|__|"#.green();
    let logo_2_1 = r#" ________  ________  ________  ___  __       "#.magenta();
    let logo_2_2 = r#"|\   __  \|\   __  \|\   ____\|\  \|\  \     "#.magenta();
    let logo_2_3 = r#"\ \  \|\  \ \  \|\  \ \  \___|\ \  \/  /|_   "#.magenta();
    let logo_2_4 = r#"\ \   ____\ \   __  \ \  \    \ \   ___  \  "#.magenta();
    let logo_2_5 = r#"\ \  \___|\ \  \ \  \ \  \____\ \  \\ \  \ "#.magenta();
    let logo_2_6 = r#" \ \__\    \ \__\ \__\ \_______\ \__\\ \__\"#.magenta();
    let logo_2_7 = r#"  \|__|     \|__|\|__|\|_______|\|__| \|__|"#.magenta();
    println!(
        "{}{}\n{}{}\n{}{}\n{}{}\n{}{}\n{}{}\n{}{}\n",
        logo_1_1,
        logo_2_1,
        logo_1_2,
        logo_2_2,
        logo_1_3,
        logo_2_3,
        logo_1_4,
        logo_2_4,
        logo_1_5,
        logo_2_5,
        logo_1_6,
        logo_2_6,
        logo_1_7,
        logo_2_7,
    )
}
