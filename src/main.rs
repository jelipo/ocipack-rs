#![feature(exclusive_range_pattern)]

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Error, Result};
use env_logger::Env;
use log::info;
use serde::Deserialize;
use tar::Builder;

use crate::progress::{Processor, ProcessResult};
use crate::progress::manager::ProcessorManager;
use crate::reg::docker::{Manifest2, ManifestLayer};
use crate::reg::docker::http::download::DownloadResult;
use crate::reg::docker::http::RegistryAuth;
use crate::reg::docker::http::upload::UploadResult;
use crate::reg::docker::image::ConfigBlob;
use crate::reg::docker::registry::Registry;
use crate::reg::home::HomeDir;
use crate::reg::{Reference, RegDigest};
use crate::util::{compress, random};
use crate::util::sha::file_sha256;

mod progress;
mod reg;
mod registry_client;
mod util;
mod bar;
mod config;

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
            username: temp_config.from.username.clone(),
            password: temp_config.from.password.clone(),
        }),
    };
    let home_dir_path = Path::new(&temp_config.home_dir);
    let home_dir = Rc::new(HomeDir::new_home_dir(home_dir_path)?);
    let mut from_registry = Registry::open(temp_config.from.registry.clone(), from_auth_opt, home_dir.clone())?;


    let from_image_reference = Reference {
        image_name: temp_config.from.image_name.as_str(),
        reference: temp_config.from.reference.as_str(),
    };
    let manifest = from_registry.image_manager.manifests(&from_image_reference)?;
    let config_digest = &manifest.config.digest;


    let mut reg_downloader_vec = Vec::<Box<dyn Processor<DownloadResult>>>::new();
    for layer in &manifest.layers {
        let digest = RegDigest::new_with_digest(layer.digest.clone());
        let downloader = from_registry.image_manager.layer_blob_download(&from_image_reference.image_name, &digest, Some(layer.size))?;
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
        let layer = layer_digest_map.get(download_result.blob_config.reg_digest.digest.as_str())
            .expect("internal error");
        if !layer_types.contains(&layer.media_type.as_str()) {
            return Err(Error::msg(format!("unknown layer media type:{}", layer.media_type)));
        }
        let digest = RegDigest::new_with_digest(layer.digest.clone());
        let _unzip_file = home_dir.cache.blobs.ungz_download_file(&digest)?;
    }

    let config_blob = from_registry.image_manager.config_blob(&temp_config.from.image_name, &config_digest)?;

    upload(home_dir.clone(), &temp_config, &config_blob, &manifest, &layer_digest_map)?;
    Ok(())
}

fn upload(
    home_dir: Rc<HomeDir>, temp_config: &TempConfig, from_config_blob: &ConfigBlob, manifest: &Manifest2,
    _layer_digest_map: &HashMap<&str, &ManifestLayer>,
) -> Result<()> {
    let tar_temp_file_path = home_dir.cache.temp_dir.join(random::random_str(10) + ".tar");
    let tar_temp_file = File::create(tar_temp_file_path.as_path())?;
    let mut tar_builder = Builder::new(tar_temp_file);
    tar_builder.append_file("root/a.txt", &mut File::open(&temp_config.test_file)?)?;
    let _tar_file = tar_builder.into_inner()?;
    let layer_info = home_dir.cache.gz_layer_file(tar_temp_file_path.as_path())?;
    info!("tgz sha256:{}", &layer_info.gz_sha256);
    info!("tar sha256:{}", &layer_info.tar_sha256);
    info!("tgz file path:{:?}", &layer_info.gz_temp_file_path);

    let to_config = &temp_config.to;
    let to_auth_opt = match to_config.username.as_str() {
        "" => None,
        _username => Some(RegistryAuth {
            username: to_config.username.clone(),
            password: to_config.password.clone(),
        }),
    };
    let mut to_registry = Registry::open(to_config.registry.clone(), to_auth_opt, home_dir.clone())?;
    let mut reg_uploader_vec = Vec::<Box<dyn Processor<UploadResult>>>::new();
    for layer in manifest.layers.iter() {
        let layer_digest = RegDigest::new_with_digest(layer.digest.clone());
        let tgz_file_path = home_dir.cache.blobs.tgz_file_path(&layer_digest)
            .expect("local download file not found");
        let file_path_str = tgz_file_path.as_os_str().to_string_lossy().to_string();
        let reg_uploader = to_registry.image_manager.layer_blob_upload(
            &to_config.image_name, &layer_digest, &file_path_str,
        )?;
        reg_uploader_vec.push(Box::new(reg_uploader))
    }
    //
    let custom_layer_uploader = to_registry.image_manager.layer_blob_upload(
        &to_config.image_name, &RegDigest::new_with_sha256(layer_info.gz_sha256.clone()),
        &layer_info.gz_temp_file_path.as_os_str().to_string_lossy().to_string(),
    )?;
    reg_uploader_vec.push(Box::new(custom_layer_uploader));

    // config blob 上传
    let mut to_config_blob = from_config_blob.clone();
    to_config_blob.rootfs.diff_ids.insert(0, format!("sha256:{}", layer_info.tar_sha256));
    let config_blob_str = serde_json::to_string(&to_config_blob)?;
    let config_blob_path = home_dir.cache.write_temp_file(config_blob_str)?;
    let config_blob_path_str = config_blob_path.as_os_str().to_string_lossy().to_string();
    let config_blob_digest = RegDigest::new_with_sha256(file_sha256(&config_blob_path)?);
    let config_blob_uploader = to_registry.image_manager.layer_blob_upload(
        &to_config.image_name, &config_blob_digest, &config_blob_path_str,
    )?;
    reg_uploader_vec.push(Box::new(config_blob_uploader));
    //
    let manager = ProcessorManager::new_processor_manager(reg_uploader_vec)?;
    let upload_results = manager.wait_all_done()?;
    for upload_result in upload_results {
        info!("{}", &upload_result.finished_info());
    }
    // layers上传完成，开始组装manifest
    let mut to_manifest = (*manifest).clone();
    to_manifest.config.digest = config_blob_digest.digest;
    to_manifest.config.media_type = "application/vnd.docker.container.image.v1+json".to_string();
    to_manifest.config.size = config_blob_path.metadata()?.len();
    // 插入一个layer
    to_manifest.layers.insert(0, ManifestLayer {
        media_type: "application/vnd.docker.image.rootfs.diff.tar.gzip".to_string(),
        size: layer_info.gz_temp_file_path.metadata()?.len(),
        digest: format!("sha256:{}", &layer_info.gz_sha256),
    });
    let put_result = to_registry.image_manager.put_manifest(&Reference {
        image_name: to_config.image_name.as_str(),
        reference: to_config.reference.as_str(),
    }, to_manifest)?;
    info!("put result: {}",put_result);
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
    test_file: String,
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
