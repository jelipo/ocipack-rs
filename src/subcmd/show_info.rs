use anyhow::{anyhow, Result};
use colored::Colorize;
use log::info;
use serde_json::Value;

use crate::adapter::docker::DockerfileAdapter;
use crate::adapter::ImageInfo;
use crate::config::cmd::{BaseAuth, ShowInfoArgs, TargetType};
use crate::config::RegAuthType;
use crate::container::image::docker::DockerConfigBlob;
use crate::container::image::oci::OciConfigBlob;
use crate::container::manifest::{Manifest, ManifestResponseEnum};
use crate::container::proxy::ProxyInfo;
use crate::container::{ConfigBlobEnum, Platform, Reference, RegContentType, Registry, RegistryCreateInfo};

pub struct ShowInfoCommand {}

impl ShowInfoCommand {
    pub fn show(show_info_args: &ShowInfoArgs) -> Result<()> {
        if let TargetType::Registry(image) = &show_info_args.image {
            let proxy = show_info_args.proxy.clone();
            let (image_info, auth) = RegistryImageInfo::gen_image_info(image, show_info_args.auth.as_ref())?;
            info!("Requesting registry...");
            let detail = RegistryImageInfo::info(
                !show_info_args.allow_insecure,
                image_info,
                auth,
                proxy,
                show_info_args.platform.clone(),
            )?;
            info!("Request done.");
            print_image_detail(detail)?;
        } else {
            return Err(anyhow!("not support {:?}", show_info_args.image));
        }
        Ok(())
    }
}

fn print_image_detail(info: ImageShowInfo) -> Result<()> {
    let manifest_pretty = serde_json::to_string_pretty(&serde_json::from_str::<Value>(&info.manifest_raw)?)?;

    let config_blob_pretty = serde_json::to_string_pretty(&serde_json::from_str::<Value>(&info.config_blob_raw)?)?;
    let vec = vec![
        ("HOST", info.image_host.green()),
        ("IMAGE_NAME", info.image.green()),
        ("IMAGE_REFERENCE", info.reference.green()),
        ("MANIFEST_TYPE", info.manifest_type.green()),
        ("OS", info.os.map(|os| os.green()).unwrap_or_else(|| "NOT SET".yellow())),
        ("ARCH", info.arch.map(|arch| arch.green()).unwrap_or_else(|| "NOT SET".yellow())),
        (
            "CMD",
            info.cmds.map(|v| format!("{:?}", v).green()).unwrap_or_else(|| "NONE".yellow()),
        ),
        (
            "ENTRYPOINT",
            info.entrypoints.map(|v| format!("{:?}", v).green()).unwrap_or_else(|| "NONE".yellow()),
        ),
    ];
    println!("\n{}\n", "IMAGE DETAILS".cyan());
    for (name, value) in vec {
        println!("{:16}: {}", name.blue(), value);
    }
    println!();

    if let Some(platforms) = info.manifest_list_platforms {
        println!(
            "{:16}:\n{}\n",
            "MANIFEST_LIST_PLATFORMS".blue(),
            platforms.iter().map(|platform| platform.to_string()).collect::<Vec<String>>().join(", ").green()
        );
    }
    if let Some(manifest_list_raw_str) = info.manifest_list_raw {
        let manifest_list_pretty = serde_json::to_string_pretty(&serde_json::from_str::<Value>(&manifest_list_raw_str)?)?;
        println!("{:16}:\n{}\n", "MANIFEST_LIST_RAW".blue(), manifest_list_pretty.green());
    }
    println!("{:16}:\n{}\n", "MANIFEST_RAW".blue(), manifest_pretty.green());
    println!("{:16}:\n{}\n", "CONFIG_BLOB_RAW".blue(), config_blob_pretty.green());
    Ok(())
}

pub struct RegistryImageInfo {}

impl RegistryImageInfo {
    /// 根据Image和Auth生成基本信息
    fn gen_image_info(image_name: &str, auth: Option<&BaseAuth>) -> Result<(ImageInfo, RegAuthType)> {
        let fake_dockerfile_body = format!("FROM {}", image_name);
        let (mut image_info, _) = DockerfileAdapter::parse_from_str(&fake_dockerfile_body)?;
        // add library
        let image_name = &image_info.image_name;
        if !image_name.contains('/') {
            image_info.image_name = format!("library/{}", image_name)
        }
        let reg_auth = RegAuthType::build_auth(image_info.image_host.clone(), auth);
        Ok((image_info, reg_auth))
    }

    /// 获取
    fn info(
        https: bool,
        image_info: ImageInfo,
        auth: RegAuthType,
        proxy: Option<ProxyInfo>,
        platform: Option<Platform>,
    ) -> Result<ImageShowInfo> {
        let info = RegistryCreateInfo {
            auth: auth.get_auth()?,
            conn_timeout_second: 600,
            proxy,
        };

        let mut registry_client = Registry::open(https, &image_info.image_host, info)?;
        let image_manager = &mut registry_client.image_manager;
        let reference = Reference {
            image_name: &image_info.image_name,
            reference: &image_info.reference,
        };
        let response = image_manager.request_manifest(
            &reference,
            &[
                RegContentType::OCI_MANIFEST,
                RegContentType::DOCKER_MANIFEST,
                RegContentType::DOCKER_MANIFEST_LIST,
                RegContentType::OCI_INDEX,
            ],
        )?;
        let mut manifest_list_raw: Option<String> = None;
        let mut manifest_list_platforms: Option<Vec<Platform>> = None;
        let (manifest, manifest_raw) = match response.manifest() {
            ManifestResponseEnum::Manifest(manifest) => (manifest.clone(), response.raw_body().to_string()),
            ManifestResponseEnum::ManifestList(manifest_list) => {
                manifest_list_raw = Some(response.raw_body().to_string());
                manifest_list_platforms = Some(manifest_list.platforms());
                let pf = platform.unwrap_or_else(|| {
                    let pf = Platform::default();
                    info!("Platform is not set, use default platform {}.", pf.to_string().green());
                    pf
                });
                image_manager.select_manifest(&reference, manifest_list, pf)?
            }
        };
        let (config_blob_enum, config_blob_raw) = match &manifest {
            Manifest::OciV1(_) => {
                let (blob, raw) = image_manager.config_blob::<OciConfigBlob>(&image_info.image_name, manifest.config_digest())?;
                (ConfigBlobEnum::OciV1(blob), raw)
            }
            Manifest::DockerV2S2(_) => {
                let (blob, raw) = image_manager.config_blob::<DockerConfigBlob>(&image_info.image_name, manifest.config_digest())?;
                (ConfigBlobEnum::DockerV2S2(blob), raw)
            }
        };
        Ok(ImageShowInfo {
            image_host: image_info.image_host,
            image: image_info.image_name,
            reference: image_info.reference,
            arch: config_blob_enum.arch().map(|a| a.to_string()),
            cmds: config_blob_enum.cmd().cloned(),
            entrypoints: config_blob_enum.entrypoint().cloned(),
            os: config_blob_enum.os().map(|a| a.to_string()),
            manifest_list_raw,
            manifest_type: manifest.manifest_type().to_string(),
            manifest_raw,
            config_blob_raw,
            manifest_list_platforms,
        })
    }
}

struct ImageShowInfo {
    image_host: String,
    image: String,
    reference: String,
    arch: Option<String>,
    cmds: Option<Vec<String>>,
    entrypoints: Option<Vec<String>>,
    os: Option<String>,
    manifest_list_raw: Option<String>,
    manifest_type: String,
    manifest_raw: String,
    config_blob_raw: String,
    manifest_list_platforms: Option<Vec<Platform>>,
}
