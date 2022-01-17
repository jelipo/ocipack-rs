use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::rc::Rc;

use anyhow::Result;
use log::info;
use tar::{Builder, Header};

use crate::progress::{Processor, ProcessResult};
use crate::progress::manager::ProcessorManager;
use crate::reg::{ConfigBlobEnum, Layer, LayerConvert, Reference, RegContentType, RegDigest, Registry};
use crate::reg::docker::image::DockerConfigBlob;
use crate::reg::home::HomeDir;
use crate::reg::http::download::DownloadResult;
use crate::reg::http::RegistryAuth;
use crate::reg::http::upload::UploadResult;
use crate::reg::manifest::{CommonManifestLayer, Manifest};
use crate::reg::oci::image::OciConfigBlob;
use crate::tempconfig::TempConfig;
use crate::util::random;
use crate::util::sha::file_sha256;

pub fn run() -> Result<()> {
    // let config_path = Path::new("config.json");
    // let config_file = File::open(config_path).expect("Open config file failed.");
    //
    // let temp_config = serde_json::from_reader::<_, TempConfig>(config_file)?;
    //
    // let from_auth_opt = match temp_config.from.username.as_str() {
    //     "" => None,
    //     _username => Some(RegistryAuth {
    //         username: temp_config.from.username.clone(),
    //         password: temp_config.from.password.clone(),
    //     }),
    // };
    // let home_dir_path = Path::new(&temp_config.home_dir);
    // let home_dir = Rc::new(HomeDir::new_home_dir(home_dir_path)?);
    //
    // let mut from_registry = Registry::open(true, &temp_config.from.registry, from_auth_opt)?;
    //
    //
    // let from_image_reference = Reference {
    //     image_name: temp_config.from.image_name.as_str(),
    //     reference: temp_config.from.reference.as_str(),
    // };
    // let manifest = from_registry.image_manager.manifests(&from_image_reference)?;
    // let (config_digest, layers) = match &manifest {
    //     Manifest::OciV1(oci) => (&oci.config.digest, oci.get_layers()),
    //     Manifest::DockerV2S2(docker) => (&docker.config.digest, docker.get_layers()),
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
    //     home_dir.cache.blobs.create_layer_config(&tar_sha256, &tar_path)?;
    // }
    //
    // let config_blob_enum = match &manifest {
    //     Manifest::OciV1(_) => ConfigBlobEnum::OciV1(from_registry.image_manager
    //         .config_blob::<OciConfigBlob>(&temp_config.from.image_name, config_digest)?),
    //     Manifest::DockerV2S2(_) => ConfigBlobEnum::DockerV2S2(from_registry.image_manager
    //         .config_blob::<DockerConfigBlob>(&temp_config.from.image_name, config_digest)?)
    // };
    //
    // upload(home_dir.clone(), &temp_config, &config_blob_enum, &manifest)?;
    Ok(())
}

fn upload(
    home_dir: Rc<HomeDir>, temp_config: &TempConfig, from_config_blob_enum: &ConfigBlobEnum, from_manifest: &Manifest,
) -> Result<()> {
    // let tar_temp_file_path = home_dir.cache.temp_dir.join(random::random_str(10) + ".tar");
    // let tar_temp_file = File::create(tar_temp_file_path.as_path())?;
    // let mut tar_builder = Builder::new(tar_temp_file);
    // let _header = Header::new_gnu();
    // tar_builder.append_file("root/a.txt", &mut File::open(&temp_config.test_file)?)?;
    // let _tar_file = tar_builder.into_inner()?;
    // let layer_info = home_dir.cache.gz_layer_file(tar_temp_file_path.as_path())?;
    // info!("tgz sha256:{}", &layer_info.tgz_sha256);
    // info!("tar sha256:{}", &layer_info.tar_sha256);
    // info!("tgz file path:{:?}", &layer_info.gz_temp_file_path);
    //
    // let to_config = &temp_config.to;
    // let to_auth_opt = match to_config.username.as_str() {
    //     "" => None,
    //     _username => Some(RegistryAuth {
    //         username: to_config.username.clone(),
    //         password: to_config.password.clone(),
    //     }),
    // };
    //
    // let mut to_registry = Registry::open(true, &to_config.registry, to_auth_opt)?;
    // let mut reg_uploader_vec = Vec::<Box<dyn Processor<UploadResult>>>::new();
    //
    // let from_layers = match &from_manifest {
    //     Manifest::OciV1(oci) => oci.get_layers(),
    //     Manifest::DockerV2S2(docker) => docker.get_layers(),
    // };
    // for layer in from_layers.iter() {
    //     let layer_digest = RegDigest::new_with_digest(layer.digest.to_string());
    //     let tgz_file_path = home_dir.cache.blobs.diff_layer_path(&layer_digest)
    //         .expect("local download file not found");
    //     let file_path_str = tgz_file_path.as_os_str().to_string_lossy().to_string();
    //     let reg_uploader = to_registry.image_manager.layer_blob_upload(
    //         &to_config.image_name, &layer_digest, &file_path_str,
    //     )?;
    //     reg_uploader_vec.push(Box::new(reg_uploader))
    // }
    // //
    // let custom_layer_uploader = to_registry.image_manager.layer_blob_upload(
    //     &to_config.image_name, &RegDigest::new_with_sha256(layer_info.tgz_sha256.clone()),
    //     &layer_info.gz_temp_file_path.as_os_str().to_string_lossy().to_string(),
    // )?;
    // reg_uploader_vec.push(Box::new(custom_layer_uploader));
    //
    // // config blob 上传
    // let to_config_blob_enum = from_config_blob_enum.clone();
    // let config_blob_digest = format!("sha256:{}", layer_info.tar_sha256);
    // // TODO 改为不同的config_blob
    // let config_blob_str = match to_config_blob_enum {
    //     ConfigBlobEnum::OciV1(mut oci_config_blob) => {
    //         oci_config_blob.rootfs.diff_ids.insert(0, config_blob_digest);
    //         serde_json::to_string(&oci_config_blob)?
    //     }
    //     ConfigBlobEnum::DockerV2S2(mut docker_config_blob) => {
    //         docker_config_blob.rootfs.diff_ids.insert(0, config_blob_digest);
    //         serde_json::to_string(&docker_config_blob)?
    //     }
    // };
    // let config_blob_path = home_dir.cache.write_temp_file(config_blob_str)?;
    // let config_blob_path_str = config_blob_path.as_os_str().to_string_lossy().to_string();
    // let config_blob_digest = RegDigest::new_with_sha256(file_sha256(&config_blob_path)?);
    // let config_blob_uploader = to_registry.image_manager.layer_blob_upload(
    //     &to_config.image_name, &config_blob_digest, &config_blob_path_str,
    // )?;
    // reg_uploader_vec.push(Box::new(config_blob_uploader));
    // //
    // let manager = ProcessorManager::new_processor_manager(reg_uploader_vec)?;
    // let upload_results = manager.wait_all_done()?;
    // for upload_result in upload_results {
    //     info!("{}", &upload_result.finished_info());
    // }
    // // layers上传完成，开始组装manifest
    // let to_type = ToType::DockerV2S2;
    // let mut to_manifest = from_manifest.clone();
    // to_manifest = match to_type {
    //     ToType::OciV1 => {
    //         let mut oci_manifest = to_manifest.to_oci_v1()?;
    //         oci_manifest.config.digest = config_blob_digest.digest;
    //         oci_manifest.config.media_type = RegContentType::OCI_IMAGE_CONFIG.val().to_string();
    //         oci_manifest.config.size = config_blob_path.metadata()?.len();
    //         oci_manifest.layers.insert(0, CommonManifestLayer {
    //             media_type: RegContentType::OCI_LAYER_TGZ.val().to_string(),
    //             size: layer_info.gz_temp_file_path.metadata()?.len(),
    //             digest: format!("sha256:{}", &layer_info.tgz_sha256),
    //         });
    //         Manifest::OciV1(oci_manifest)
    //     }
    //     ToType::DockerV2S2 => {
    //         let mut docker_manifest = to_manifest.to_docker_v2_s2()?;
    //         docker_manifest.config.digest = config_blob_digest.digest;
    //         docker_manifest.config.media_type = RegContentType::DOCKER_CONTAINER_IMAGE.val().to_string();
    //         docker_manifest.config.size = config_blob_path.metadata()?.len();
    //         docker_manifest.layers.insert(0, CommonManifestLayer {
    //             media_type: RegContentType::DOCKER_LAYER_TGZ.val().to_string(),
    //             size: layer_info.gz_temp_file_path.metadata()?.len(),
    //             digest: format!("sha256:{}", &layer_info.tgz_sha256),
    //         });
    //         Manifest::DockerV2S2(docker_manifest)
    //     }
    // };
    // let put_result = to_registry.image_manager.put_manifest(&Reference {
    //     image_name: to_config.image_name.as_str(),
    //     reference: to_config.reference.as_str(),
    // }, to_manifest)?;
    // info!("put result: {}",put_result);
    Ok(())
}

fn layer_to_map<'a>(layers: &'a Vec<Layer>) -> HashMap<&'a str, &'a Layer<'a>> {
    let mut map = HashMap::<&str, &Layer>::with_capacity(layers.len());
    for layer in layers {
        map.insert(&layer.digest, layer);
    }
    map
}

pub enum ToType {
    OciV1,
    DockerV2S2,
}