use anyhow::{Error, Result};
use dockerfile_parser::{Dockerfile, Instruction};
use log::info;

use crate::adapter::{ImageInfo, TargetImageAdapter, TargetInfo};
use crate::config::cmd::{BaseAuth, TargetFormat};
use crate::config::RegAuthType;
use crate::GLOBAL_CONFIG;
use crate::progress::manager::ProcessorManager;
use crate::progress::Processor;
use crate::progress::ProcessResult;
use crate::reg::{ConfigBlobEnum, Reference, RegDigest, Registry};
use crate::reg::http::upload::UploadResult;
use crate::reg::manifest::Manifest;
use crate::util::sha::file_sha256;

pub struct RegistryTargetAdapter {
    info: TargetInfo,
    use_https: bool,
    target_manifest: Manifest,
    target_config_blob: ConfigBlobEnum,
    target_auth: RegAuthType,
}

impl TargetImageAdapter for RegistryTargetAdapter {
    fn info(&self) -> &TargetInfo {
        &self.info
    }
}

impl RegistryTargetAdapter {
    pub fn new(
        image: &str,
        format: TargetFormat,
        use_https: bool,
        target_manifest: Manifest,
        target_config_blob: ConfigBlobEnum,
        base_auth: Option<&BaseAuth>,
    ) -> Result<RegistryTargetAdapter> {
        let temp_from = format!("FROM {}", image);
        let instruction = Dockerfile::parse(&temp_from)?.instructions.remove(0);
        let image_info = match instruction {
            Instruction::From(from) => ImageInfo {
                image_host: from.image_parsed.registry,
                image_name: from.image_parsed.image,
                reference: from.image_parsed.tag.or(from.image_parsed.hash)
                    .ok_or_else(|| Error::msg("can not found hash or tag"))?,
            },
            _ => return Err(Error::msg("image info error")),
        };
        let auth = RegAuthType::build_auth(image_info.image_host.as_ref(), base_auth);
        Ok(RegistryTargetAdapter {
            info: TargetInfo {
                image_info,
                format,
            },
            use_https,
            target_manifest,
            target_config_blob,
            target_auth: auth,
        })
    }

    pub fn upload(self) -> Result<()> {
        let home_dir = GLOBAL_CONFIG.home_dir.clone();
        let target_info = self.info;
        let reg_auth = self.target_auth.get_auth()?;
        let host = target_info.image_info.image_host
            .unwrap_or_else(|| "registry-1.docker.io/v2".to_string());
        let target_reg = Registry::open(self.use_https, &host, reg_auth)?;
        let mut manager = target_reg.image_manager;

        let target_manifest = self.target_manifest;
        let mut reg_uploader_vec = Vec::<Box<dyn Processor<UploadResult>>>::new();
        for manifest_layer in target_manifest.layers() {
            let layer_digest = RegDigest::new_with_digest(manifest_layer.digest.to_string());
            let local_layer = home_dir.cache.blobs.local_layer(&layer_digest)
                .ok_or_else(|| Error::msg("local download file not found"))?;
            let layer_path = local_layer.layer_path();
            let reg_uploader = manager.layer_blob_upload(
                &target_info.image_info.image_name, &layer_digest, &layer_path,
            )?;
            reg_uploader_vec.push(Box::new(reg_uploader))
        }

        let config_blob_str = self.target_config_blob.to_json_string()?;

        let config_blob_path = home_dir.cache.write_temp_file(config_blob_str)?;
        let config_blob_path_str = config_blob_path.to_string_lossy().to_string();
        let config_blob_digest = RegDigest::new_with_sha256(file_sha256(&config_blob_path)?);
        let config_blob_uploader = manager.layer_blob_upload(
            &target_info.image_info.image_name, &config_blob_digest, &config_blob_path_str,
        )?;
        reg_uploader_vec.push(Box::new(config_blob_uploader));
        //
        let process_manager = ProcessorManager::new_processor_manager(reg_uploader_vec)?;
        let upload_results = process_manager.wait_all_done()?;
        for upload_result in upload_results {
            info!("upload done : {}", &upload_result.finished_info());
        }

        let _put_result = manager.put_manifest(&Reference {
            image_name: target_info.image_info.image_name.as_str(),
            reference: target_info.image_info.reference.as_str(),
        }, target_manifest)?;

        Ok(())
    }
}