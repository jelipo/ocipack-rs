use std::fs::File;
use std::rc::Rc;

use anyhow::{Error, Result};
use dockerfile_parser::{Dockerfile, FromInstruction, Instruction};
use log::info;
use tar::{Builder, Header};

use crate::adapter::{ImageInfo, TargetImageAdapter, TargetInfo};
use crate::config::cmd::{TargetFormat, TargetType};
use crate::config::RegAuthType;
use crate::docker::ToType;
use crate::GLOBAL_CONFIG;
use crate::progress::manager::ProcessorManager;
use crate::progress::Processor;
use crate::progress::ProcessResult;
use crate::reg::{ConfigBlobEnum, Layer, LayerConvert, Reference, RegContentType, RegDigest, Registry};
use crate::reg::home::{HomeDir, LayerInfo};
use crate::reg::http::RegistryAuth;
use crate::reg::http::upload::UploadResult;
use crate::reg::manifest::{CommonManifestLayer, Manifest};
use crate::tempconfig::TempConfig;
use crate::util::file::remove;
use crate::util::random;
use crate::util::sha::file_sha256;

pub struct RegistryTargetAdapter {
    info: TargetInfo,
    use_https: bool,
}

impl TargetImageAdapter for RegistryTargetAdapter {
    fn info(&self) -> &TargetInfo {
        &self.info
    }
}

impl RegistryTargetAdapter {
    pub fn new(image: &str, format: TargetFormat, use_https: bool) -> Result<RegistryTargetAdapter> {
        let temp_from = format!("FROM {}", image);
        let instruction = Dockerfile::parse(&temp_from)?.instructions.remove(0);
        let image_info = match instruction {
            Instruction::From(from) => ImageInfo {
                image_host: from.image_parsed.registry,
                image_name: from.image_parsed.image,
                reference: from.image_parsed.tag.or(from.image_parsed.hash)
                    .ok_or(Error::msg("can not found hash or tag"))?,
            },
            _ => return Err(Error::msg("")),
        };
        Ok(RegistryTargetAdapter {
            info: TargetInfo {
                image_info,
                format,
            },
            use_https,
        })
    }
}

fn upload(use_https: bool, info: TargetInfo, auth: RegAuthType, source_manifest: &Manifest, new_layers: Vec<LayerInfo>) -> Result<()> {
    let home_dir = GLOBAL_CONFIG.home_dir.clone();
    let reg_auth = auth.get_auth()?;
    let host = info.image_info.image_host.unwrap_or("registry-1.docker.io/v2".to_string());
    let mut target_reg = Registry::open(use_https, &host, reg_auth)?;
    let mut manager = target_reg.image_manager;
    let source_layers = source_manifest.layers();

    let mut reg_uploader_vec = Vec::<Box<dyn Processor<UploadResult>>>::new();

    for layer in source_layers.iter() {
        let layer_digest = RegDigest::new_with_digest(layer.digest.to_string());
        let tgz_file_path = home_dir.cache.blobs.diff_layer_path(&layer_digest)
            .expect("local download file not found");
        let file_path_str = tgz_file_path.as_os_str().to_string_lossy().to_string();
        let reg_uploader = manager.layer_blob_upload(
            &info.image_info.image_name, &layer_digest, &file_path_str,
        )?;
        reg_uploader_vec.push(Box::new(reg_uploader))
    }
    for new_layer in new_layers {
        let custom_layer_uploader = manager.layer_blob_upload(
            &info.image_info.image_name, &RegDigest::new_with_sha256(new_layer.gz_sha256.clone()),
            &new_layer.gz_temp_file_path.as_os_str().to_string_lossy().to_string(),
        )?;
        reg_uploader_vec.push(Box::new(custom_layer_uploader));
    }

    let manager = ProcessorManager::new_processor_manager(reg_uploader_vec)?;
    let upload_results = manager.wait_all_done()?;
    for upload_result in upload_results {
        info!("{}", &upload_result.finished_info());
    }


    Ok(())
}


