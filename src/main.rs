#![feature(exclusive_range_pattern)]

use std::fs::File;

use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use serde::Deserialize;
use crate::progress::manager::ProcessorManager;
use crate::progress::Processor;

use crate::reg::http::RegistryAuth;
use crate::reg::image::ManifestLayer;
use crate::reg::registry::Registry;
use crate::reg::Reference;

mod reg;
mod registry_client;
mod util;
mod display;
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
    let vec = manifest.layers;

    // ProcessorManager::new_processor_manager()

    for layer in vec {
        download(&layer, &mut registry, &reference)?;
    }
    //download(&vec[0], &mut registry, &reference)?;
    sleep(Duration::from_secs(60 * 5));
    Ok(())
}

fn download(
    manifest_layer: &ManifestLayer,
    registry: &mut Registry,
    reference: &Reference,
) -> Result<()> {
    let disgest = &manifest_layer.digest;
    let downloader_opt = registry.image_manager.blobs_download(&reference.image_name, disgest)?;
    if let Some(downloader) = downloader_opt {
        let handle = downloader.start();
        let arc = downloader.process_status();
        loop {
            sleep(Duration::from_secs(1));
            let status = arc.lock().unwrap();
            if status.is_done() {
                println!("下载完成");
                break;
            } else {
                println!("{}", status.now_size());
            }
        }
        let result = handle.wait_result().unwrap();
        println!("文件路径：{}", result);
    } else {
        println!("无需下载");
    }
    return Ok(());
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
