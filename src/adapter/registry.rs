use anyhow::{anyhow, Result};
use dockerfile_parser::{Dockerfile, Instruction};
use log::{debug, info};
use reqwest::StatusCode;

use crate::adapter::{ImageInfo, TargetImageAdapter, TargetInfo};
use crate::config::cmd::{BaseAuth, TargetFormat};
use crate::config::RegAuthType;
use crate::const_data::DEFAULT_IMAGE_HOST;
use crate::progress::manager::ProcessorManager;
use crate::progress::ProcessResult;
use crate::progress::Processor;
use crate::reg::http::upload::UploadResult;
use crate::reg::manifest::Manifest;
use crate::reg::proxy::ProxyInfo;
use crate::reg::{ConfigBlobSerialize, Reference, RegDigest, Registry, RegistryCreateInfo};
use crate::GLOBAL_CONFIG;

pub struct RegistryTargetAdapter {
    info: TargetInfo,
    use_https: bool,
    conn_timeout_second: u64,
    target_manifest: Manifest,
    target_config_blob_serialize: ConfigBlobSerialize,
    target_auth: RegAuthType,
    target_proxy: Option<ProxyInfo>,
}

impl TargetImageAdapter for RegistryTargetAdapter {
    fn info(&self) -> &TargetInfo {
        &self.info
    }
}

impl RegistryTargetAdapter {
    pub fn new(
        image_raw: &str,
        format: TargetFormat,
        use_https: bool,
        target_manifest: Manifest,
        target_config_blob_serialize: ConfigBlobSerialize,
        base_auth: Option<&BaseAuth>,
        conn_timeout_second: u64,
        target_proxy: Option<ProxyInfo>,
    ) -> Result<RegistryTargetAdapter> {
        let temp_from = format!("FROM {}", image_raw);
        let instruction = Dockerfile::parse(&temp_from)?.instructions.remove(0);
        let image_info = match instruction {
            Instruction::From(from) => ImageInfo {
                image_host: from.image_parsed.registry.unwrap_or_else(|| DEFAULT_IMAGE_HOST.to_string()),
                image_name: if from.image_parsed.image.contains('/') {
                    from.image_parsed.image
                } else {
                    format!("library/{}", from.image_parsed.image)
                },
                reference: from.image_parsed.tag.or(from.image_parsed.hash).unwrap_or_else(|| "latest".to_string()),
            },
            _ => return Err(anyhow!("image info error")),
        };
        let auth = RegAuthType::build_auth(image_info.image_host.clone(), base_auth);
        Ok(RegistryTargetAdapter {
            info: TargetInfo { image_info, format },
            use_https,
            conn_timeout_second,
            target_manifest,
            target_config_blob_serialize,
            target_auth: auth,
            target_proxy,
        })
    }

    pub fn upload(self) -> Result<()> {
        let home_dir = GLOBAL_CONFIG.home_dir.clone();
        let target_info = self.info;
        let reg_auth = self.target_auth.get_auth()?;
        let host = target_info.image_info.image_host;
        let create_info = RegistryCreateInfo {
            auth: reg_auth,
            conn_timeout_second: self.conn_timeout_second,
            proxy: self.target_proxy,
        };
        let target_reg = Registry::open(self.use_https, &host, create_info)?;
        let mut manager = target_reg.image_manager;

        let target_manifest = self.target_manifest;
        let mut reg_uploader_vec = Vec::<Box<dyn Processor<UploadResult>>>::new();
        for manifest_layer in target_manifest.layers() {
            let layer_digest = RegDigest::new_with_digest(manifest_layer.digest.to_string());
            let local_layer =
                home_dir.cache.blobs.local_layer(&layer_digest).ok_or_else(|| anyhow!("local download file not found"))?;
            let layer_path = local_layer.layer_path();
            let reg_uploader = manager.layer_blob_upload(&target_info.image_info.image_name, &layer_digest, &layer_path)?;
            reg_uploader_vec.push(Box::new(reg_uploader))
        }
        let serialize = self.target_config_blob_serialize;
        let config_blob_str = serialize.json_str;

        let config_blob_path = home_dir.cache.write_temp_file(config_blob_str)?;
        let config_blob_path_str = config_blob_path.to_string_lossy().to_string();
        let config_blob_uploader =
            manager.layer_blob_upload(&target_info.image_info.image_name, &serialize.digest, &config_blob_path_str)?;
        reg_uploader_vec.push(Box::new(config_blob_uploader));
        //
        let process_manager = ProcessorManager::new_processor_manager(reg_uploader_vec)?;
        let upload_results = process_manager.wait_all_done()?;
        for upload_result in upload_results {
            debug!("upload done : {}", &upload_result.finished_info());
        }
        let (status_code, body) = manager.put_manifest(
            &Reference {
                image_name: target_info.image_info.image_name.as_str(),
                reference: target_info.image_info.reference.as_str(),
            },
            target_manifest,
        )?;
        if StatusCode::from_u16(status_code).is_ok() {
            info!("upload image success");
        } else {
            info!("upload image failed\nHTTP code:{}\nbody:{}", status_code, body);
        }
        Ok(())
    }
}
