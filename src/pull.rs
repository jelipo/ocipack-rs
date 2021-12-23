use std::path::Path;
use std::rc::Rc;
use crate::adapter::SourceImageAdapter;
use crate::config::RegAuthType;
use crate::reg::home::HomeDir;
use crate::reg::Registry;

pub fn pull(
    source_adapter: Box<dyn SourceImageAdapter>,
    source_auth: RegAuthType,
    home_dir: Rc<HomeDir>,
    use_https: bool,
) {
    let info1 = source_adapter.into_info();
    // source_adapter.info().image_info.image_host.unwrap_or("")
    // let mut from_registry = Registry::open(use_https, , from_auth_opt, home_dir.clone())?;
    //
    //
    // let from_image_reference = Reference {
    //     image_name: temp_config.from.image_name.as_str(),
    //     reference: temp_config.from.reference.as_str(),
    // };
    // let manifest = from_registry.image_manager.manifests(&from_image_reference)?;
    // let (config_digest, layers) = match &manifest {
    //     Manifest::OciV1(oci) => (&oci.config.digest, oci.to_layers()),
    //     Manifest::DockerV2S2(docker) => (&docker.config.digest, docker.to_layers()),
    // };
    //
    // let mut reg_downloader_vec = Vec::<Box<dyn Processor<DownloadResult>>>::new();
    // for layer in &layers {
    //     let digest = RegDigest::new_with_digest(layer.digest.to_string());
    //     let downloader = from_registry.image_manager.layer_blob_download(&from_image_reference.image_name, &digest, Some(layer.size))?;
    //     reg_downloader_vec.push(Box::new(downloader))
    // }
    // info!("创建manager");
    // let manager = ProcessorManager::new_processor_manager(reg_downloader_vec)?;
    // let download_results = manager.wait_all_done()?;
    // let layer_digest_map = layer_to_map(&layers);
    // for download_result in &download_results {
    //     if download_result.local_existed {
    //         continue;
    //     }
    //     let layer = layer_digest_map.get(download_result.blob_config.reg_digest.digest.as_str())
    //         .expect("internal error");
    //     let digest = RegDigest::new_with_digest(layer.digest.to_string());
    //     let (tar_sha256, tar_path) = home_dir.cache.blobs.ungz_download_file(&digest)?;
    //     home_dir.cache.blobs.create_tar_shafile(&tar_sha256, &tar_path)?;
    // }
    //
    // let config_blob_enum = match &manifest {
    //     Manifest::OciV1(_) => ConfigBlobEnum::OciV1(from_registry.image_manager
    //         .config_blob::<OciConfigBlob>(&temp_config.from.image_name, config_digest)?),
    //     Manifest::DockerV2S2(_) => ConfigBlobEnum::DockerV2S2(from_registry.image_manager
    //         .config_blob::<DockerConfigBlob>(&temp_config.from.image_name, config_digest)?)
    // };
}