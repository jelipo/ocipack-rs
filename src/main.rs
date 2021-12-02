#![feature(exclusive_range_pattern)]

use std::fs::File;
use std::path::Path;

use anyhow::Result;
use env_logger::Env;
use log::info;
use serde::Deserialize;

use crate::progress::manager::ProcessorManager;
use crate::progress::Processor;
use crate::reg::{BlobType, Reference};
use crate::reg::docker::http::RegistryAuth;
use crate::reg::docker::registry::Registry;

mod progress;
mod reg;
mod registry_client;
mod util;
mod bar;

fn main() -> Result<()> {
    let env = Env::default()
        .default_filter_or("info");
    env_logger::init_from_env(env);

    let config_path = Path::new("config.json");
    let config_file = File::open(config_path).expect("Open config file failed.");

    let temp_config = serde_json::from_reader::<_, TempConfig>(config_file)?;

    let from_auth_opt = match temp_config.from_config.username.as_str() {
        "" => None,
        _username => Some(RegistryAuth {
            username: temp_config.from_config.username,
            password: temp_config.from_config.password,
        }),
    };
    let home_dir_path = Path::new(&temp_config.from_config.home_dir);
    let mut from_registry = Registry::open(temp_config.from_config.registry, from_auth_opt, home_dir_path)?;

    from_registry.image_manager.layer_blob_upload(&temp_config.from_config.image_name, "sha256:7b1a6ab2e44dbac178598dabe7cff59bd67233dba0b27e4fbd1f9d4b3c877a53", "")?;

    let frome_image_reference = Reference {
        image_name: temp_config.from_config.image_name.as_str(),
        reference: temp_config.from_config.reference.as_str(),
    };
    let manifest = from_registry.image_manager.manifests(&frome_image_reference)?;
    let config_digest = &manifest.config.digest;


    let mut reg_downloader_vec = Vec::<Box<dyn Processor<String>>>::new();
    for layer in &manifest.layers {
        let downloader = from_registry.image_manager.layer_blob_download(&frome_image_reference.image_name, &layer.digest)?;
        reg_downloader_vec.push(Box::new(downloader))
    }
    info!("创建manager");
    let manager = ProcessorManager::new_processor_manager(reg_downloader_vec)?;
    manager.wait_all_done()?;

    let _config_blob = from_registry.image_manager.config_blob(&temp_config.from_config.image_name, &config_digest)?;
    info!("全部下载完成");
    Ok(())
}

#[derive(Deserialize)]
struct TempConfig {
    from_config: FromConfig,
    to_config: ToConfig,
}

#[derive(Deserialize)]
struct FromConfig {
    registry: String,
    image_name: String,
    reference: String,
    username: String,
    password: String,
    home_dir: String,
}

#[derive(Deserialize)]
struct ToConfig {
    registry: String,
    image_name: String,
    reference: String,
    username: String,
    password: String,
    home_dir: String,
}
