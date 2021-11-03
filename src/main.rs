#![feature(exclusive_range_pattern)]

use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use serde::Deserialize;

use crate::reg::http::RegistryAuth;
use crate::reg::image::ManifestLayer;
use crate::reg::Reference;
use crate::reg::registry::Registry;

mod reg;
mod registry_client;
mod util;

fn main() -> Result<()> {
    let config_path = Path::new("config.json");
    let mut config_file = File::open(config_path)?;
    let temp_config = serde_json::from_reader::<_, TempConfig>(config_file)?;
    let auth = RegistryAuth {
        username: temp_config.username,
        password: temp_config.password,
    };
    let home_dir_path = Path::new(temp_config.home_dir.as_str());
    let mut registry = Registry::open(temp_config.registry, Some(auth), home_dir_path)?;
    let reference = Reference {
        image_name: temp_config.image_name.as_str(),
        reference: temp_config.reference.as_str(),
    };
    let manifest = registry.image_manager.manifests(&reference)?;
    let vec = manifest.layers;
    let layer = &vec[0];
    download(layer, &registry, &reference)?;
    sleep(Duration::from_secs(60 * 5));
    Ok(())
}

fn download(
    manifest_layer: &ManifestLayer,
    registry: &Registry,
    reference: &Reference,
) -> Result<()> {
    let disgest = &manifest_layer.digest;
    let mut downloader_opt = registry.image_manager.blobs_download(&reference.image_name, disgest)?;
    if let Some(downloader) = downloader_opt {
        let handle = downloader.start()?;
        let arc = downloader.download_temp();
        loop {
            sleep(Duration::from_secs(1));
            let temp = arc.lock().unwrap();
            if temp.done {
                println!("下载完成");
                break;
            } else {
                println!("{}", temp.curr_size);
            }
        }
        let result = handle.join().unwrap();
        println!("文件路径：{}",result.unwrap());
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