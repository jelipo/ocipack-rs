use std::collections::HashMap;
use std::fs::File;

use anyhow::{anyhow, Result};
use log::info;
use sha2::{Digest, Sha256};

use crate::adapter::SourceInfo;
use crate::config::RegAuthType;
use crate::progress::manager::ProcessorManager;
use crate::progress::Processor;
use crate::reg::docker::image::DockerConfigBlob;
use crate::reg::http::download::DownloadResult;
use crate::reg::manifest::Manifest;
use crate::reg::oci::image::OciConfigBlob;
use crate::reg::proxy::ProxyInfo;
use crate::reg::{ConfigBlobEnum, Layer, Reference, RegContentType, RegDigest, Registry, RegistryCreateInfo};
use crate::util::compress::uncompress;
use crate::GLOBAL_CONFIG;

pub fn pull(
    source_info: &SourceInfo,
    source_auth: RegAuthType,
    use_https: bool,
    read_timeout_second: u64,
    proxy: Option<ProxyInfo>,
) -> Result<PullResult> {
    let image_info = &source_info.image_info;
    let image_host = &image_info.image_host;
    let from_image_reference = Reference {
        image_name: &image_info.image_name,
        reference: image_info.reference.as_str(),
    };
    info!("From image info. host='{}' name='{}' reference='{}'",image_host,&image_info.image_name,image_info.reference);
    let registry_auth = source_auth.get_auth()?;
    let info = RegistryCreateInfo {
        auth: registry_auth,
        conn_timeout_second: read_timeout_second,
        proxy,
    };
    let mut from_registry = Registry::open(use_https, image_host, info)?;
    info!("Get from image manifest info.");
    let manifest = from_registry.image_manager.manifests(&from_image_reference)?;
    info!("From image type: {}", match manifest {
        Manifest::OciV1(_) => "OCI",
        Manifest::DockerV2S2(_) => "Docker V2,Schema2"
    });
    let config_digest = manifest.config_digest();
    let layers = manifest.layers();
    let mut reg_downloader_vec = Vec::<Box<dyn Processor<DownloadResult>>>::new();
    for layer in &layers {
        let digest = RegDigest::new_with_digest(layer.digest.to_string());
        let downloader =
            from_registry.image_manager.layer_blob_download(from_image_reference.image_name, &digest, Some(layer.size))?;
        reg_downloader_vec.push(Box::new(downloader))
    }
    let manager = ProcessorManager::new_processor_manager(reg_downloader_vec)?;
    info!("Start pulling... (total={})", manager.size());
    let download_results = manager.wait_all_done()?;
    let layer_digest_map = layer_to_map(&layers);
    for download_result in &download_results {
        if download_result.local_existed {
            continue;
        }
        let manifest_layer =
            layer_digest_map.get(download_result.blob_config.reg_digest.digest.as_str()).expect("internal error");
        let layer_compress_type = RegContentType::compress_type(manifest_layer.media_type)?;
        let digest = RegDigest::new_with_digest(manifest_layer.digest.to_string());
        let download_path = download_result.file_path.as_ref().ok_or_else(|| anyhow!("can not found download file"))?;
        // 计算解压完的tar的sha256值
        let mut download_file = File::open(download_path)?;
        let mut sha256_encode = Sha256::new();
        uncompress(layer_compress_type, &mut download_file, &mut sha256_encode)?;
        let sha256 = &sha256_encode.finalize()[..];
        let tar_sha256 = hex::encode(sha256);
        GLOBAL_CONFIG.home_dir.cache.blobs.create_layer_config(&tar_sha256, &digest.sha256, layer_compress_type)?;
        GLOBAL_CONFIG.home_dir.cache.blobs.move_to_blob(download_path, &digest.sha256, &tar_sha256)?;
    }

    let config_blob_enum = match &manifest {
        Manifest::OciV1(_) => ConfigBlobEnum::OciV1(
            from_registry.image_manager.config_blob::<OciConfigBlob>(&image_info.image_name, config_digest)?,
        ),
        Manifest::DockerV2S2(_) => ConfigBlobEnum::DockerV2S2(
            from_registry.image_manager.config_blob::<DockerConfigBlob>(&image_info.image_name, config_digest)?,
        ),
    };

    Ok(PullResult {
        config_blob: config_blob_enum,
        manifest,
    })
}

fn layer_to_map<'a>(layers: &'a [Layer]) -> HashMap<&'a str, &'a Layer<'a>> {
    let mut map = HashMap::<&str, &Layer>::with_capacity(layers.len());
    for layer in layers {
        map.insert(layer.digest, layer);
    }
    map
}

pub struct PullResult {
    pub config_blob: ConfigBlobEnum,
    pub manifest: Manifest,
}
