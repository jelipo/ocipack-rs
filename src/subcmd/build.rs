use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use colored::Colorize;
use log::info;
use tar::Builder;

use crate::adapter::docker::DockerfileAdapter;
use crate::adapter::registry::RegistryTargetAdapter;
use crate::adapter::tar::TarTargetAdapter;
use crate::adapter::{BuildInfo, CopyFile, SourceInfo};
use crate::config::cmd::{BuildCmdArgs, SourceType, TargetFormat, TargetType};
use crate::config::RegAuthType;
use crate::container::home::{LocalLayer, TempLayerInfo};
use crate::container::manifest::Manifest;
use crate::container::proxy::ProxyInfo;
use crate::container::{CompressType, ConfigBlobEnum, ConfigBlobSerialize};
use crate::subcmd::pull::pull;
use crate::util::sha::{Sha256Reader, Sha256Writer};
use crate::util::{compress, random};
use crate::{HomeDir, GLOBAL_CONFIG};

pub struct BuildCommand {}

impl BuildCommand {
    pub fn build(build_args: &BuildCmdArgs) -> Result<()> {
        let (source_info, build_info, source_auth) = build_source_info(build_args)?;
        match handle(
            source_info,
            build_info,
            source_auth,
            build_args,
            build_args.source_proxy.clone(),
            build_args.use_zstd,
        ) {
            Ok(_) => print_build_success(build_args),
            Err(err) => print_build_failed(err),
        }
        Ok(())
    }
}

fn print_build_success(build_args: &BuildCmdArgs) {
    println!(
        "{}",
        format!(
            r#"
Build job successful!

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

fn print_build_failed(err: anyhow::Error) {
    println!(
        "{}",
        format!(
            r#"
Build job failed!

{}
"#,
            err
        )
            .red()
    );
}

fn build_source_info(build_args: &BuildCmdArgs) -> Result<(SourceInfo, BuildInfo, RegAuthType)> {
    let (mut image_info, build_info) = match &build_args.source {
        SourceType::Dockerfile { path } => DockerfileAdapter::parse(path)?,
        SourceType::Cmd { tag: _ } => {
            todo!()
        }
        SourceType::Registry { image } => {
            let fake_dockerfile_body = format!("FROM {}", image);
            DockerfileAdapter::parse_from_str(&fake_dockerfile_body)?
        },
    };
    // add library
    let image_name = &image_info.image_name;
    if !image_name.contains('/') {
        image_info.image_name = format!("library/{}", image_name)
    }
    let source_reg_auth = RegAuthType::build_auth(image_info.image_host.clone(), build_args.source_auth.as_ref());
    Ok((
        SourceInfo {
            image_info,
            platform: build_args.platform.clone(),
        },
        build_info,
        source_reg_auth,
    ))
}

fn handle(
    source_info: SourceInfo,
    build_info: BuildInfo,
    source_auth: RegAuthType,
    build_cmds: &BuildCmdArgs,
    proxy_info: Option<ProxyInfo>,
    use_zstd: bool,
) -> Result<()> {
    let home_dir = GLOBAL_CONFIG.home_dir.clone();
    let pull_result = pull(
        &source_info,
        source_auth,
        !build_cmds.allow_insecure,
        build_cmds.conn_timeout,
        proxy_info,
    )?;
    let compress_type = if use_zstd { CompressType::Zstd } else { CompressType::Tgz };
    let temp_layer = build_top_tar(&build_info.copy_files, &home_dir)?
        .map(|tar_path| compress_layer_file(&tar_path, &home_dir, compress_type))
        .transpose()?;
    let temp_local_layer = if let Some(temp_layer) = &temp_layer {
        home_dir.cache.blobs.move_to_blob(
            &temp_layer.compress_layer_path,
            &temp_layer.compressed_tar_sha256,
            &temp_layer.tar_sha256,
        )?;
        let temp_local_layer =
            home_dir.cache.blobs.create_layer_config(&temp_layer.tar_sha256, &temp_layer.compressed_tar_sha256, compress_type)?;
        Some(temp_local_layer)
    } else {
        None
    };
    let target_config_blob =
        build_target_config_blob(build_info, &pull_result.config_blob, temp_layer.as_ref(), &build_cmds.format);
    let source_manifest = pull_result.manifest;
    let source_manifest_raw = pull_result.manifest_raw;
    let target_config_blob_serialize = target_config_blob.serialize()?;
    info!("Build a new target manifest.");
    let target_manifest = build_target_manifest(
        source_manifest,
        &build_cmds.format,
        temp_local_layer,
        &target_config_blob_serialize,
    )?;
    match &build_cmds.target {
        TargetType::Registry(image) => {
            let registry_adapter = RegistryTargetAdapter::new(
                image,
                build_cmds.format.clone(),
                !build_cmds.target_allow_insecure,
                target_manifest,
                target_config_blob_serialize,
                build_cmds.target_auth.as_ref(),
                build_cmds.conn_timeout,
                build_cmds.target_proxy.clone(),
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

/// 构建一个tar layer
fn build_top_tar(copyfiles: &[CopyFile], home_dir: &HomeDir) -> Result<Option<PathBuf>> {
    if copyfiles.is_empty() {
        return Ok(None);
    }
    info!("Building new tar...");
    let tar_file_name = random::random_str(10) + ".tar";
    let tar_temp_file_path = home_dir.cache.temp_dir.join(tar_file_name);
    let tar_temp_file = File::create(tar_temp_file_path.as_path())?;
    let mut tar_builder = Builder::new(tar_temp_file);
    for copyfile in copyfiles {
        for source_path_str in &copyfile.source_path {
            let source_path = PathBuf::from(&source_path_str);
            if !source_path.exists() {
                return Err(anyhow!("path not found:{}", source_path_str));
            }
            let dest_path = if copyfile.dest_path.ends_with('/') {
                &copyfile.dest_path[1..]
            } else {
                &copyfile.dest_path
            };
            if source_path.is_file() {
                let file_name = source_path.file_name().ok_or_else(|| anyhow!("error file name"))?.to_string_lossy();
                let dest_file_path = PathBuf::from(dest_path).join(file_name.to_string()).to_string_lossy().to_string();
                let mut sourcefile = File::open(source_path)?;
                tar_builder.append_file(dest_file_path, &mut sourcefile)?;
            } else if source_path.is_dir() {
                tar_builder.append_dir(dest_path, source_path_str)?;
            } else {
                return Err(anyhow!("copy only support file and dir".to_string()));
            }
        }
    }
    tar_builder.finish()?;
    info!("Build tar complete");
    Ok(Some(tar_temp_file_path))
}

/// 压缩tar layer文件为指定格式
fn compress_layer_file(tar_file_path: &Path, home_dir: &HomeDir, compress_type: CompressType) -> Result<TempLayerInfo> {
    let tar_file = File::open(tar_file_path)?;
    let mut sha256_reader = Sha256Reader::new(tar_file);
    let compress_file_name = random::random_str(20) + ".compress";
    let compress_file_path = home_dir.cache.temp_dir.join(compress_file_name);
    let compress_file = File::create(&compress_file_path)?;
    let mut sha256_writer = Sha256Writer::new(compress_file);
    info!("Compressing tar...  (compress-type={})", compress_type.to_string());
    compress::compress(compress_type, &mut sha256_reader, &mut sha256_writer)?;
    let tar_sha256 = sha256_reader.sha256()?;
    let compressed_tar_sha256 = sha256_writer.sha256()?;
    info!("Compress complete. (sha256={})", compressed_tar_sha256);
    Ok(TempLayerInfo {
        compressed_tar_sha256,
        tar_sha256,
        compress_layer_path: compress_file_path,
        compress_type,
    })
}

pub fn build_target_config_blob(
    build_info: BuildInfo,
    source_config_blob: &ConfigBlobEnum,
    temp_layer_opt: Option<&TempLayerInfo>,
    target_format: &TargetFormat,
) -> ConfigBlobEnum {
    // TODO change to diff config type
    let mut target_config_blob = match target_format {
        TargetFormat::Docker => source_config_blob.clone(),
        TargetFormat::Oci => source_config_blob.clone(),
    };
    if let Some(temp_layer) = temp_layer_opt {
        let new_tar_digest = format!("sha256:{}", temp_layer.tar_sha256);
        target_config_blob.add_diff_layer(new_tar_digest);
        // TODO add a history
    }
    target_config_blob.add_labels(build_info.labels);
    target_config_blob.add_envs(build_info.envs);
    if let Some(cmds) = build_info.cmd {
        target_config_blob.overwrite_cmd(cmds)
    }
    if let Some(port_exposes) = build_info.ports {
        target_config_blob.add_ports(port_exposes);
    }
    if let Some(work_dir) = build_info.workdir {
        target_config_blob.overwrite_work_dir(work_dir);
    }
    if let Some(user) = build_info.user {
        target_config_blob.overwrite_user(user);
    }
    target_config_blob
}

pub fn build_target_manifest(
    source_manifest: Manifest,
    target_format: &TargetFormat,
    temp_local_layer: Option<LocalLayer>,
    target_config_blob_serialize: &ConfigBlobSerialize,
) -> Result<Manifest> {
    let mut target_manifest = match target_format {
        TargetFormat::Docker => Manifest::DockerV2S2(source_manifest.to_docker_v2_s2(target_config_blob_serialize)?),
        TargetFormat::Oci => Manifest::OciV1(source_manifest.to_oci_v1(target_config_blob_serialize)?),
    };
    if let Some(temp_layer) = temp_local_layer {
        let metadata = temp_layer.layer_file_path.metadata()?;
        target_manifest.add_top_layer(metadata.len(), temp_layer.manifest_sha, temp_layer.compress_type)?
    }
    Ok(target_manifest)
}
