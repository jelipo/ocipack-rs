use anyhow::Result;

use crate::adapter::{Adapter, FromImageAdapter};
use crate::config::{BaseImage, RegAuthType};

pub struct DockerfileAdapter {
    docker_file_path: String,
    image_host: String,
    image_name: String,
    /// 可以是TAG或者digest
    reference: String,
    /// 验证方式
    auth_type: RegAuthType,
}

impl Adapter for DockerfileAdapter {
    fn image_info(&self) -> Result<BaseImage> {
        Ok(BaseImage {
            use_https: true,
            reg_host: "".to_string(),
            image_name: "".to_string(),
            reference: "".to_string(),
            auth_type: self.auth_type.clone(),
        })
    }
}

impl FromImageAdapter for DockerfileAdapter {
    fn new_envs(&self) -> Option<&[String]> {
        todo!()
    }

    fn new_cmds(&self) -> Option<&[String]> {
        todo!()
    }
}