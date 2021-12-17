use std::env;
use std::lazy::OnceCell;

use anyhow::Result;
use env_logger::Env;

use crate::GlobalAppConfig;

//pub static GLOBAL_CONFIG: OnceCell<GlobalAppConfig> = OnceCell::new();


pub fn init() -> Result<()> {
    log_init();


    Ok(())
}

fn log_init() {
    let env = Env::default()
        .default_filter_or("info");
    env_logger::init_from_env(env);
}

fn cmd() {

}