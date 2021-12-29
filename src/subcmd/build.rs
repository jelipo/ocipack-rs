use anyhow::Result;

use crate::adapter::{BuildInfo, SourceImageAdapter, SourceInfo, TargetImageAdapter};
use crate::adapter::docker::DockerfileAdapter;
use crate::adapter::registry::RegistryTargetAdapter;
use crate::config::cmd::{BaseAuth, BuildArgs, SourceType, TargetType};
use crate::config::RegAuthType;
use crate::GLOBAL_CONFIG;
use crate::subcmd::pull::pull;

pub struct BuildCommand {}

impl BuildCommand {
    pub fn build(build_args: &BuildArgs) -> Result<()> {
        let (source_info, build_info, source_auth) = build_source_info(build_args)?;

        Ok(())
    }
}

fn build_source_info(build_args: &BuildArgs) -> Result<(SourceInfo, BuildInfo, RegAuthType)> {
    let (source_info, build_info) = match &build_args.source {
        SourceType::Dockerfile { path } => DockerfileAdapter::parse(path)?,
        SourceType::Cmd { tag: _ } => { todo!() }
    };
    let source_reg_auth = build_auth(source_info.image_info.image_host.as_ref(),
                                     build_args.source_auth.as_ref());
    Ok((source_info, build_info, source_reg_auth))
}


fn build_auth(image_host: Option<&String>, base_auth: Option<&BaseAuth>) -> RegAuthType {
    match base_auth.as_ref() {
        None => RegAuthType::LocalDockerAuth {
            reg_host: image_host.map(|s| s.as_str())
                .unwrap_or("https://index.docker.io/v1/").to_string()
        },
        Some(auth) => RegAuthType::CustomPassword {
            username: auth.username.clone(),
            password: auth.password.clone(),
        }
    }
}

fn handle(
    source_info: SourceInfo,
    build_info: BuildInfo,
    source_auth: RegAuthType,
    build_args: &BuildArgs,
) -> Result<()> {
    let home_dir = GLOBAL_CONFIG.home_dir.clone();
    let pull = pull(&source_info, source_auth, !build_args.allow_insecure)?;

    Ok(())
}