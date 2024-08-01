use anyhow::Result;

use crate::adapter::{BuildInfo, ImageInfo, SourceInfo};
use crate::adapter::docker::DockerfileAdapter;
use crate::config::cmd::{BaseAuth, SourceType};
use crate::config::RegAuthType;
use crate::container::Platform;

pub mod build;
pub mod clean;
pub mod pull;
pub mod show_info;
pub mod transform;
mod sync;



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