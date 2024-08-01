use anyhow::{anyhow, Result};

use crate::adapter::docker::DockerfileAdapter;
use crate::adapter::SourceInfo;
use crate::config::cmd::{BaseAuth, SourceType, SyncCmdArgs};
use crate::config::RegAuthType;
use crate::container::{Platform, Reference, RegContentType, Registry, RegistryCreateInfo};
use crate::GLOBAL_CONFIG;
use crate::subcmd::build_source_info;
use crate::subcmd::pull::pull;

pub struct SyncInfoCommand {}

impl SyncInfoCommand {
    pub fn sync(sync_arg: &SyncCmdArgs) -> Result<()> {
        let (source_info, source_auth) =
            sync_source_info(&sync_arg.source, sync_arg.source_auth.as_ref(), sync_arg.platform.clone())?;
        let source_image = match &sync_arg.source {
            SourceType::Registry { image } => image,
            _ => return Err(anyhow!("sync not support {:?}", sync_arg.source)),
        };
        Ok(())
    }
}

// 
fn sync_source_info(source: &SourceType, source_auth: Option<&BaseAuth>, platform: Option<Platform>) -> Result<(SourceInfo, RegAuthType)> {
    let mut image_info = match source {
        SourceType::Registry { image } => {
            let fake_dockerfile_body = format!("FROM {}", image);
            DockerfileAdapter::parse_from_str(&fake_dockerfile_body)?.0
        }
        _ => return Err(anyhow!("not support {:?}", source)),
    };
    // add library
    let image_name = &image_info.image_name;
    if !image_name.contains('/') {
        image_info.image_name = format!("library/{}", image_name)
    }
    let source_reg_auth = RegAuthType::build_auth(image_info.image_host.clone(), source_auth);
    Ok((
        SourceInfo { image_info, platform: platform.clone() },
        source_reg_auth,
    ))
}

fn handle_source(source_info: SourceInfo, source_auth: RegAuthType, cmds: &SyncCmdArgs) -> Result<()> {
    let info = RegistryCreateInfo {
        auth: source_auth.get_auth()?,
        conn_timeout_second: cmds.conn_timeout,
        proxy: cmds.source_proxy.clone(),
    };
    let home_dir = GLOBAL_CONFIG.home_dir.clone();
    let mut registry_client = Registry::open(cmds.allow_insecure, &source_info.image_info.image_host, info)?;
    let image_manager = &mut registry_client.image_manager;
    let reference = Reference {
        image_name: &source_info.image_info.image_name,
        reference: &source_info.image_info.reference,
    };
    let response = image_manager.request_manifest(
        &reference, &[
            RegContentType::OCI_MANIFEST,
            RegContentType::DOCKER_MANIFEST,
            RegContentType::DOCKER_MANIFEST_LIST,
            RegContentType::OCI_INDEX,
        ],
    )?;
    let pull_result = pull(&source_info, source_auth.clone(), cmds.allow_insecure, cmds.conn_timeout, cmds.source_proxy.clone())?;
    
    Ok(())
}