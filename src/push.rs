use std::fs::File;
use std::rc::Rc;

use anyhow::Result;
use log::info;
use tar::{Builder, Header};

use crate::config::cmd::{TargetFormat, TargetType};
use crate::docker::ToType;
use crate::progress::manager::ProcessorManager;
use crate::progress::Processor;
use crate::progress::ProcessResult;
use crate::reg::{ConfigBlobEnum, LayerConvert, Reference, RegContentType, RegDigest, Registry};
use crate::reg::home::HomeDir;
use crate::reg::http::RegistryAuth;
use crate::reg::http::upload::UploadResult;
use crate::reg::manifest::{CommonManifestLayer, Manifest};
use crate::tempconfig::TempConfig;
use crate::util::random;
use crate::util::sha::file_sha256;

fn upload(
    home_dir: Rc<HomeDir>, temp_config: &TempConfig, from_config_blob_enum: &ConfigBlobEnum, from_manifest: &Manifest,
) -> Result<()> {
    let tar_temp_file_path = home_dir.cache.temp_dir.join(random::random_str(10) + ".tar");
    let tar_temp_file = File::create(tar_temp_file_path.as_path())?;
    let mut tar_builder = Builder::new(tar_temp_file);
    let header = Header::new_gnu();
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

    let mut to_registry = Registry::open(true, &to_config.registry, to_auth_opt)?;
    let mut reg_uploader_vec = Vec::<Box<dyn Processor<UploadResult>>>::new();

    let from_layers = match &from_manifest {
        Manifest::OciV1(oci) => oci.get_layers(),
        Manifest::DockerV2S2(docker) => docker.get_layers(),
    };
    for layer in from_layers.iter() {
        let layer_digest = RegDigest::new_with_digest(layer.digest.to_string());
        let tgz_file_path = home_dir.cache.blobs.diff_layer_path(&layer_digest)
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
    let to_config_blob_enum = from_config_blob_enum.clone();
    let config_blob_digest = format!("sha256:{}", layer_info.tar_sha256);
    // TODO 改为不同的config_blob
    let config_blob_str = match to_config_blob_enum {
        ConfigBlobEnum::OciV1(mut oci_config_blob) => {
            oci_config_blob.rootfs.diff_ids.insert(0, config_blob_digest);
            serde_json::to_string(&oci_config_blob)?
        }
        ConfigBlobEnum::DockerV2S2(mut docker_config_blob) => {
            docker_config_blob.rootfs.diff_ids.insert(0, config_blob_digest);
            serde_json::to_string(&docker_config_blob)?
        }
    };
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
    let to_type = ToType::DockerV2S2;
    let mut to_manifest = from_manifest.clone();
    to_manifest = match to_type {
        ToType::OciV1 => {
            let mut oci_manifest = to_manifest.to_oci_v1()?;
            oci_manifest.config.digest = config_blob_digest.digest;
            oci_manifest.config.media_type = RegContentType::OCI_IMAGE_CONFIG.val().to_string();
            oci_manifest.config.size = config_blob_path.metadata()?.len();
            oci_manifest.layers.insert(0, CommonManifestLayer {
                media_type: RegContentType::OCI_LAYER_TGZ.val().to_string(),
                size: layer_info.gz_temp_file_path.metadata()?.len(),
                digest: format!("sha256:{}", &layer_info.gz_sha256),
            });
            Manifest::OciV1(oci_manifest)
        }
        ToType::DockerV2S2 => {
            let mut docker_manifest = to_manifest.to_docker_v2_s2()?;
            docker_manifest.config.digest = config_blob_digest.digest;
            docker_manifest.config.media_type = RegContentType::DOCKER_CONTAINER_IMAGE.val().to_string();
            docker_manifest.config.size = config_blob_path.metadata()?.len();
            docker_manifest.layers.insert(0, CommonManifestLayer {
                media_type: RegContentType::DOCKER_LAYER_TGZ.val().to_string(),
                size: layer_info.gz_temp_file_path.metadata()?.len(),
                digest: format!("sha256:{}", &layer_info.gz_sha256),
            });
            Manifest::DockerV2S2(docker_manifest)
        }
    };
    let put_result = to_registry.image_manager.put_manifest(&Reference {
        image_name: to_config.image_name.as_str(),
        reference: to_config.reference.as_str(),
    }, to_manifest)?;
    info!("put result: {}",put_result);
    Ok(())
}