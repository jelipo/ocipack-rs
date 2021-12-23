use anyhow::Result;

use crate::adapter::SourceImageAdapter;
use crate::adapter::docker::DockerfileAdapter;
use crate::adapter::registry::RegistryTargetAdapter;
use crate::config::cmd::{BaseAuth, BuildArgs, SourceType, TargetType};
use crate::config::RegAuthType;

pub struct BuildCommand {}

impl BuildCommand {
    pub fn build(build_args: &BuildArgs) -> Result<()> {
        let from_adapter = build_from_info(build_args)?;
        Ok(())
    }
}

fn build_from_info(build_args: &BuildArgs) -> Result<Box<dyn SourceImageAdapter>> {
    let from_adapter: Box<dyn SourceImageAdapter> = match &build_args.source {
        SourceType::Dockerfile { path } => Box::new(
            DockerfileAdapter::new(path)?
        ),
        SourceType::Cmd { tag: _ } => { todo!() }
    };
    let auth_host = from_adapter.info().image_info.image_host.as_ref()
        .map(|s| s.as_str()).unwrap_or("https://index.docker.io/v1/");
    let from_reg_auth = match build_args.source_auth.as_ref() {
        None => RegAuthType::LocalDockerAuth { reg_host: auth_host.to_string() },
        Some(auth) => RegAuthType::CustomPassword {
            username: auth.username.clone(),
            password: auth.password.clone(),
        }
    };
    Ok(from_adapter)
}


fn build_target_info(build_args: &BuildArgs) -> Result<Box<dyn ToImageAdapter>> {
    let to_adapter: Box<dyn ToImageAdapter> = match &build_args.target {
        TargetType::Registry(image) => Box::new(RegistryTargetAdapter::new(image)?)
    };
    let auth_host = from_adapter.info().image_info.image_host.as_ref()
        .map(|s| s.as_str()).unwrap_or("https://index.docker.io/v1/");
    let target_reg_auth = match build_args.target_auth.as_ref() {
        None => RegAuthType::LocalDockerAuth { reg_host: auth_host.to_string() },
        Some(auth) => RegAuthType::CustomPassword {
            username: auth.username.clone(),
            password: auth.password.clone(),
        }
    };
    Ok(to_adapter)
}