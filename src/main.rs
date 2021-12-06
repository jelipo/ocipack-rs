#![feature(exclusive_range_pattern)]

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Error, Result};
use env_logger::Env;
use flate2::read::GzDecoder;
use log::{error, info};
use serde::Deserialize;

use crate::progress::manager::ProcessorManager;
use crate::progress::Processor;
use crate::reg::{BlobType, Reference};
use crate::reg::docker::http::download::DownloadResult;
use crate::reg::docker::http::RegistryAuth;
use crate::reg::docker::ManifestLayer;
use crate::reg::docker::registry::Registry;
use crate::reg::home::HomeDir;

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

    let from_auth_opt = match temp_config.from.username.as_str() {
        "" => None,
        _username => Some(RegistryAuth {
            username: temp_config.from.username,
            password: temp_config.from.password,
        }),
    };
    let home_dir_path = Path::new(&temp_config.home_dir);
    let home_dir = Rc::new(HomeDir::new_home_dir(home_dir_path));
    let mut from_registry = Registry::open(temp_config.from.registry, from_auth_opt, home_dir.clone())?;

    let from_image_reference = Reference {
        image_name: temp_config.from.image_name.as_str(),
        reference: temp_config.from.reference.as_str(),
    };
    let manifest = from_registry.image_manager.manifests(&from_image_reference)?;
    let config_digest = &manifest.config.digest;


    let mut reg_downloader_vec = Vec::<Box<dyn Processor<DownloadResult>>>::new();
    for layer in &manifest.layers {
        let downloader = from_registry.image_manager.layer_blob_download(&from_image_reference.image_name, &layer.digest, Some(layer.size))?;
        reg_downloader_vec.push(Box::new(downloader))
    }
    info!("创建manager");
    let manager = ProcessorManager::new_processor_manager(reg_downloader_vec)?;
    let download_results = manager.wait_all_done()?;
    let layer_digest_map = layer_to_map(&manifest.layers);
    let layer_types = vec!["application/vnd.docker.image.rootfs.foreign.diff.tar.gzip",
                           "application/vnd.docker.image.rootfs.diff.tar.gzip"];
    for download_result in &download_results {
        if download_result.local_existed {
            continue;
        }
        let layer = layer_digest_map.get(download_result.blob_config.digest.as_str()).expect("internal error");
        if !layer_types.contains(&layer.media_type.as_str()) {
            return Err(Error::msg(format!("unknown layer media type:{}", layer.media_type)));
        }
        let unzip_file = home_dir.cache.blobs.ungzip_download_file(&layer.digest)?;
    }

    let _config_blob = from_registry.image_manager.config_blob(&temp_config.from.image_name, &config_digest)?;
    info!("全部下载完成");
    Ok(())
}

fn layer_to_map(layers: &Vec<ManifestLayer>) -> HashMap<&str, &ManifestLayer> {
    let mut map = HashMap::<&str, &ManifestLayer>::with_capacity(layers.len());
    for layer in layers {
        map.insert(&layer.digest, layer);
    }
    map
}

#[derive(Deserialize)]
struct TempConfig {
    from: FromConfig,
    to: ToConfig,
    home_dir: String,
}

#[derive(Deserialize)]
struct FromConfig {
    registry: String,
    image_name: String,
    reference: String,
    username: String,
    password: String,

}

#[derive(Deserialize)]
struct ToConfig {
    registry: String,
    image_name: String,
    reference: String,
    username: String,
    password: String,
}
