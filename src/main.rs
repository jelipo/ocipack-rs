#![feature(exclusive_range_pattern)]

use std::fs::File;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use serde::Deserialize;

use crate::progress::manager::ProcessorManager;
use crate::progress::Processor;
use crate::reg::http::download::RegDownloader;
use crate::reg::http::RegistryAuth;
use crate::reg::image::ManifestLayer;
use crate::reg::Reference;
use crate::reg::registry::Registry;

mod reg;
mod registry_client;
mod util;
mod progress;

fn main() -> Result<()> {
    let config_path = Path::new("config.json");
    let config_file = File::open(config_path)
        .expect("Open config file failed.");
    let temp_config = serde_json::from_reader::<_, TempConfig>(config_file)?;
    let auth_opt = match temp_config.username.as_str() {
        "" => None,
        _ => Some(RegistryAuth {
            username: temp_config.username,
            password: temp_config.password,
        })
    };
    let home_dir_path = Path::new(temp_config.home_dir.as_str());
    let mut registry = Registry::open(temp_config.registry, auth_opt, home_dir_path)?;
    let reference = Reference {
        image_name: temp_config.image_name.as_str(),
        reference: temp_config.reference.as_str(),
    };
    let manifest = registry.image_manager.manifests(&reference)?;
    let manifest_layers = manifest.layers;

    let mut reg_downloader_vec = Vec::<Box<dyn Processor<String>>>::new();
    for layer in manifest_layers {
        let disgest = &layer.digest;
        let downloader = registry.image_manager.blobs_download(&reference.image_name, disgest)?;
        reg_downloader_vec.push(Box::new(downloader))
    }
    println!("创建manager");
    let manager = ProcessorManager::new_processor_manager(reg_downloader_vec)?;
    manager.wait_all_done();

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
