#![feature(exclusive_range_pattern)]

use std::fs::File;
use std::path::Path;

use anyhow::Result;
use env_logger::Env;
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
    let auth_opt = match temp_config.username.as_str() {
        "" => None,
        _username => Some(RegistryAuth {
            username: temp_config.username,
            password: temp_config.password,
        }),
    };
    let home_dir_path = Path::new(temp_config.home_dir.as_str());
    let mut registry = Registry::open(temp_config.registry, auth_opt, home_dir_path)?;

    registry.image_manager.layer_blob_upload(temp_config.image_name.as_str(),
                                             "sha256:7b1a6ab2e44dbac178598dabe7cff59bd67233dba0b27e4fbd1f9d4b3c877a53",
                                             "")?;


    let reference = Reference {
        image_name: temp_config.image_name.as_str(),
        reference: temp_config.reference.as_str(),
    };
    let manifest = registry.image_manager.manifests(&reference)?;
    let config_digest = &manifest.config.digest;
    let _config_blob = registry.image_manager.config_blob(temp_config.image_name.as_str(), config_digest.as_str())?;

    let manifest_layers = &manifest.layers;

    let mut reg_downloader_vec = Vec::<Box<dyn Processor<String>>>::new();
    for layer in manifest_layers {
        let disgest = &layer.digest;
        let downloader = registry.image_manager.layer_blob_download(&reference.image_name, disgest)?;
        reg_downloader_vec.push(Box::new(downloader))
    }
    println!("创建manager");
    let manager = ProcessorManager::new_processor_manager(reg_downloader_vec)?;
    manager.wait_all_done()?;
    println!("全部下载完成");
    Ok(())
}

#[derive(Deserialize)]
struct TempConfig {
    registry: String,
    image_name: String,
    reference: String,
    username: String,
    password: String,
    home_dir: String,
}
