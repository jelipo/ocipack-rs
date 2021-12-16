use std::path::Path;

use anyhow::Result;

use crate::adapter::{Adapter, FromImageAdapter};
use crate::config::BaseImage;

pub struct DockerfileAdapter {
    docker_file_path: String,
}

impl Adapter for DockerfileAdapter {
    fn image_info(&self) -> Result<BaseImage> {

    }
}

impl FromImageAdapter for DockerfileAdapter {}