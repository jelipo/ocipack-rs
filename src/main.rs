#![feature(exclusive_range_pattern)]

use anyhow::Result;
use env_logger::Env;
use log::Level::Info;

mod progress;
mod reg;
mod util;
mod bar;
mod config;
mod docker;
mod tempconfig;
mod adapter;

fn main() -> Result<()> {
    let env = Env::default()
        .default_filter_or(Info.as_str());
    env_logger::init_from_env(env);


    docker::run()
}