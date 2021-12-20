use anyhow::Result;

use crate::config::BaseImage;

pub mod docker;

pub trait Adapter {
    /// 镜像信息
    fn image_info(&self) -> Result<BaseImage>;
}

pub trait FromImageAdapter: Adapter {
    /// 获取环境
    fn new_envs(&self) -> Option<&[String]>;
    /// 覆盖的Cmd
    fn new_cmds(&self) -> Option<&[String]>;

}

pub trait ToImageAdapter: Adapter {}
