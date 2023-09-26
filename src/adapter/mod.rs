use std::collections::HashMap;

use crate::config::cmd::TargetFormat;
use crate::container::Platform;

pub mod docker;
pub mod registry;
pub mod tar;

pub struct ImageInfo {
    pub image_raw_name: Option<String>,
    pub image_host: String,
    pub image_name: String,
    pub reference: String,
}

pub struct SourceInfo {
    pub image_info: ImageInfo,
    pub platform: Option<Platform>,
}

pub struct BuildInfo {
    pub labels: HashMap<String, String>,
    pub envs: HashMap<String, String>,
    pub user: Option<String>,
    pub workdir: Option<String>,
    pub cmd: Option<Vec<String>>,
    pub copy_files: Vec<CopyFile>,
    pub ports: Option<Vec<String>>,
}

pub struct TargetInfo {
    pub image_info: ImageInfo,
    pub format: TargetFormat,
}

pub trait SourceImageAdapter {
    fn info(&self) -> &SourceInfo;

    fn into_info(self: Box<Self>) -> SourceInfo;
}

pub trait TargetImageAdapter {
    fn info(&self) -> &TargetInfo;
}

pub struct CopyFile {
    pub source_path: Vec<String>,
    pub dest_path: String,
}
