use anyhow::Result;

use crate::adapter::{BuildInfo, ImageInfo, SourceInfo};
use crate::adapter::docker::DockerfileAdapter;
use crate::config::cmd::{BaseAuth, ShowInfoArgs, TargetType};
use crate::config::RegAuthType;
use crate::reg::proxy::ProxyInfo;
use crate::reg::{ConfigBlobEnum, Reference, Registry, RegistryCreateInfo};
use crate::reg::docker::image::DockerConfigBlob;
use crate::reg::manifest::Manifest;
use crate::reg::oci::image::OciConfigBlob;

pub struct ShowInfoCommand {}

impl ShowInfoCommand {
    pub fn show(show_info_args: &ShowInfoArgs) -> Result<()> {
        match &show_info_args.image {
            TargetType::Registry(image) => {
                let (image_info, auth) = RegistryImageInfo::gen_image_info(image, show_info_args.auth.as_ref())?;
            }
        }
        Ok(())
    }
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
    fn info(use_https: bool, image_info: ImageInfo, auth: RegAuthType, proxy: Option<ProxyInfo>) -> Result<()> {
        let info = RegistryCreateInfo {
            auth: auth.get_auth()?,
            conn_timeout_second: 600,
            proxy,
        };
        let mut registry_client = Registry::open(use_https, &image_info.image_host, info)?;
        let (manifest, manifest_raw) = registry_client.image_manager.manifests(&Reference {
            image_name: &image_info.image_name,
            reference: &image_info.reference,
        })?;
        let config_blob_enum = match &manifest {
            Manifest::OciV1(_) => ConfigBlobEnum::OciV1(
                registry_client.image_manager.config_blob::<OciConfigBlob>(&image_info.image_name, manifest.config_digest())?,
            ),
            Manifest::DockerV2S2(_) => ConfigBlobEnum::DockerV2S2(
                registry_client.image_manager.config_blob::<DockerConfigBlob>(&image_info.image_name, manifest.config_digest())?,
            ),
        };
        Ok(())
    }
}

struct ImageShowInfo {
    manifest_type: String,
    manifest_raw: String,
}




