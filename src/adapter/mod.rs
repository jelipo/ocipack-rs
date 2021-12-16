pub mod docker;

use anyhow::Result;

use crate::config::BaseImage;

pub trait Adapter {
    /// 镜像信息
    fn image_info(&self) -> Result<BaseImage>;
}

pub trait FromImageAdapter: Adapter {
    fn
}

pub trait ToImageAdapter: Adapter {}
