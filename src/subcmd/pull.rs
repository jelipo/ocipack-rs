use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use anyhow::Result;
use log::info;

use crate::adapter::{SourceImageAdapter, SourceInfo};
use crate::config::RegAuthType;
use crate::GLOBAL_CONFIG;
use crate::progress::manager::ProcessorManager;
use crate::progress::Processor;
use crate::reg::{ConfigBlobEnum, Layer, LayerConvert, Reference, RegDigest, Registry};
use crate::reg::docker::image::DockerConfigBlob;
use crate::reg::home::HomeDir;
use crate::reg::http::download::DownloadResult;
use crate::reg::manifest::Manifest;
use crate::reg::oci::image::OciConfigBlob;

pub fn pull(
    source_info: &SourceInfo,
    source_auth: RegAuthType,
    use_https: bool,
) -> Result<()> {
    let image_info = &source_info.image_info;
    let image_host = image_info.image_host.clone()
        .unwrap_or("registry-1.docker.io/v2".into());

    let registry_auth = source_auth.get_auth()?;
    let mut from_registry = Registry::open(use_https, &image_host, registry_auth)?;

    let from_image_reference = Reference {
        image_name: image_info.image_name.as_str(),
        reference: image_info.reference.as_str(),
    };
    let manifest = from_registry.image_manager.manifests(&from_image_reference)?;
    let (config_digest, layers) = match &manifest {
        Manifest::OciV1(oci) => (&oci.config.digest, oci.get_layers()),
        Manifest::DockerV2S2(docker) => (&docker.config.digest, docker.get_layers()),
    };

    let mut reg_downloader_vec = Vec::<Box<dyn Processor<DownloadResult>>>::new();
    for layer in &layers {
        let digest = RegDigest::new_with_digest(layer.digest.to_string());
        let downloader = from_registry.image_manager.layer_blob_download(&from_image_reference.image_name, &digest, Some(layer.size))?;
        reg_downloader_vec.push(Box::new(downloader))
    }

    let manager = ProcessorManager::new_processor_manager(reg_downloader_vec)?;
    let download_results = manager.wait_all_done()?;
    let layer_digest_map = layer_to_map(&layers);
    for download_result in &download_results {
        if download_result.local_existed {
            continue;
        }
        let layer = layer_digest_map.get(download_result.blob_config.reg_digest.digest.as_str())
            .expect("internal error");
        let digest = RegDigest::new_with_digest(layer.digest.to_string());
        let (tar_sha256, tar_path) = GLOBAL_CONFIG.home_dir.cache.blobs.ungz_download_file(&digest)?;
        GLOBAL_CONFIG.home_dir.cache.blobs.create_tar_shafile(&tar_sha256, &tar_path)?;
    }

    let config_blob_enum = match &manifest {
        Manifest::OciV1(_) => ConfigBlobEnum::OciV1(from_registry.image_manager
            .config_blob::<OciConfigBlob>(&image_info.image_name, config_digest)?),
        Manifest::DockerV2S2(_) => ConfigBlobEnum::DockerV2S2(from_registry.image_manager
            .config_blob::<DockerConfigBlob>(&image_info.image_name, config_digest)?)
    };
    Ok(())
}


fn layer_to_map<'a>(layers: &'a Vec<Layer>) -> HashMap<&'a str, &'a Layer<'a>> {
    let mut map = HashMap::<&str, &Layer>::with_capacity(layers.len());
    for layer in layers {
        map.insert(&layer.digest, layer);
    }
    map
}