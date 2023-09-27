use anyhow::{anyhow, Result};
use colored::Colorize;
use log::info;
use std::path::PathBuf;

use crate::adapter::docker::DockerfileAdapter;
use crate::adapter::registry::RegistryTargetAdapter;
use crate::adapter::tar::TarTargetAdapter;
use crate::adapter::{BuildInfo, SourceInfo};
use crate::config::cmd::{TargetType, TransformCmdArgs};
use crate::config::RegAuthType;
use crate::container::proxy::ProxyInfo;
use crate::subcmd::build::{build_target_config_blob, build_target_manifest};
use crate::subcmd::pull::pull;
use crate::GLOBAL_CONFIG;

pub struct TransformCommand {}

impl TransformCommand {
    pub fn transform(transform_args: &TransformCmdArgs) -> Result<()> {
        let (source_info, build_info, source_auth) = gen_source_info(transform_args)?;
        match transform_handle(
            source_info,
            build_info,
            source_auth,
            transform_args,
            transform_args.source_proxy.clone(),
        ) {
            Ok(_) => print_transform_success(transform_args),
            Err(err) => print_transform_failed(err),
        }
        Ok(())
    }
}

fn print_transform_success(build_args: &TransformCmdArgs) {
    println!(
        "{}",
        format!(
            r#"
Transform job successful!

Target image:
{}
"#,
            match &build_args.target {
                TargetType::Registry(r) => (*r).clone(),
                TargetType::Tar(tar_arg) => format!("Path: {}", tar_arg.path),
            }
        )
        .green()
    );
}

fn print_transform_failed(err: anyhow::Error) {
    println!(
        "{}",
        format!(
            r#"
Transform job failed!

{}
"#,
            err
        )
        .red()
    );
}

fn gen_source_info(transform_args: &TransformCmdArgs) -> Result<(SourceInfo, BuildInfo, RegAuthType)> {
    let fake_dockerfile_body = format!("FROM {}", &transform_args.source_image);
    let (mut image_info, build_info) = DockerfileAdapter::parse_from_str(&fake_dockerfile_body)?;
    // add library
    let image_name = &image_info.image_name;
    if !image_name.contains('/') {
        image_info.image_name = format!("library/{}", image_name)
    }
    let source_reg_auth = RegAuthType::build_auth(image_info.image_host.clone(), transform_args.source_auth.as_ref());
    Ok((
        SourceInfo {
            image_info,
            platform: None,
        },
        build_info,
        source_reg_auth,
    ))
}

pub fn transform_handle(
    source_info: SourceInfo,
    build_info: BuildInfo,
    source_auth: RegAuthType,
    transform_cmds: &TransformCmdArgs,
    proxy_info: Option<ProxyInfo>,
) -> Result<()> {
    let _home_dir = GLOBAL_CONFIG.home_dir.clone();
    let pull_result = pull(
        &source_info,
        source_auth,
        !transform_cmds.allow_insecure,
        transform_cmds.conn_timeout,
        proxy_info,
    )?;
    let target_config_blob = build_target_config_blob(build_info, &pull_result.config_blob, None, &transform_cmds.format);
    let source_manifest = pull_result.manifest;
    let source_manifest_raw = pull_result.manifest_raw;
    let target_config_blob_serialize = target_config_blob.serialize()?;
    info!("Build a new target manifest.");
    let target_manifest = build_target_manifest(source_manifest, &transform_cmds.format, None, &target_config_blob_serialize)?;
    match &transform_cmds.target {
        TargetType::Registry(image) => {
            let registry_adapter = RegistryTargetAdapter::new(
                image,
                transform_cmds.format.clone(),
                !transform_cmds.target_allow_insecure,
                target_manifest,
                target_config_blob_serialize,
                transform_cmds.target_auth.as_ref(),
                transform_cmds.conn_timeout,
                transform_cmds.target_proxy.clone(),
            )?;
            registry_adapter.upload()?
        }
        TargetType::Tar(tar_arg) => {
            let image_raw_name = source_info.image_info.image_raw_name.ok_or_else(|| anyhow!("must set a raw name"))?;
            let adapter = TarTargetAdapter {
                image_raw_name,
                target_manifest,
                manifest_raw: source_manifest_raw,
                target_config_blob_serialize,
                save_path: PathBuf::from(tar_arg.path.clone()),
                use_gzip: tar_arg.usb_gzip,
            };
            adapter.save()?;
        }
    }
    Ok(())
}
