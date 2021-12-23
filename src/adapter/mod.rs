use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::config::BaseImage;

pub mod docker;
pub mod registry;

pub struct ImageInfo {
    pub image_host: Option<String>,
    pub image_name: String,
    pub reference: String,
}

pub struct SourceInfo {
    pub image_info: ImageInfo,
    pub labels: HashMap<String, String>,
    pub envs: HashMap<String, String>,
    pub user: Option<String>,
    pub workdir: Option<String>,
    pub cmd: Option<Vec<String>>,
    pub copy_files: Vec<CopyFile>,
    pub ports: Vec<String>,
}

pub struct TargetInfo {
    pub image_info: ImageInfo,
}


pub trait SourceImageAdapter {
    fn info(&self) -> &SourceInfo;

    fn into_info(self) -> SourceInfo;
}

pub trait TargetImageAdapter {
    fn info(&self) -> &TargetInfo;
}

pub struct CopyFile {
    source_path: Vec<String>,
    dest_path: String,
}
