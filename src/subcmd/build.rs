use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{Error, Result};
use tar::Builder;

use crate::{GLOBAL_CONFIG, HomeDir};
use crate::adapter::{BuildInfo, CopyFile, SourceInfo};
use crate::adapter::docker::DockerfileAdapter;
use crate::adapter::registry::RegistryTargetAdapter;
use crate::config::cmd::{BuildCmdArgs, SourceType, TargetFormat, TargetType};
use crate::config::RegAuthType;
use crate::reg::{CompressType, ConfigBlobEnum};
use crate::reg::home::TempLayerInfo;
use crate::reg::manifest::Manifest;
use crate::subcmd::pull::pull;
use crate::util::{compress, random};
use crate::util::sha::{Sha256Reader, Sha256Writer};

pub struct BuildCommand {}

impl BuildCommand {
    pub fn build(build_args: &BuildCmdArgs) -> Result<()> {
        let (source_info, build_info, source_auth) = build_source_info(build_args)?;
        handle(source_info, build_info, source_auth, build_args)?;
        Ok(())
    }
}

fn build_source_info(build_args: &BuildCmdArgs) -> Result<(SourceInfo, BuildInfo, RegAuthType)> {
    let (mut source_info, build_info) = match &build_args.source {
        SourceType::Dockerfile { path } => DockerfileAdapter::parse(path)?,
        SourceType::Cmd { tag: _ } => { todo!() }
    };
    // add library
    let image_name = &source_info.image_info.image_name;
    if !image_name.contains('/') {
        source_info.image_info.image_name = format!("library/{}", image_name)
    }
    let source_reg_auth = RegAuthType::build_auth(
        source_info.image_info.image_host.as_ref(), build_args.source_auth.as_ref());
    Ok((source_info, build_info, source_reg_auth))
}

fn handle(
    source_info: SourceInfo,
    build_info: BuildInfo,
    source_auth: RegAuthType,
    build_cmds: &BuildCmdArgs,
) -> Result<()> {
    let home_dir = GLOBAL_CONFIG.home_dir.clone();
    let pull_result = pull(&source_info, source_auth, !build_cmds.allow_insecure)?;

    let temp_layer = build_top_tar(&build_info.copy_files, &home_dir)?.map(|tar_path| {
        gz_layer_file(&tar_path, &home_dir)
    }).transpose()?;
    if let Some(temp_layer) = &temp_layer {
        home_dir.cache.blobs.move_to_blob(
            &temp_layer.compress_layer_path, &temp_layer.tgz_sha256, &temp_layer.tar_sha256)?;
        let _local_layer = home_dir.cache.blobs.create_layer_config(
            &temp_layer.tar_sha256, &temp_layer.tgz_sha256, CompressType::Tgz)?;
    }
    let target_config_blob = build_target_config_blob(
        build_info, &pull_result.config_blob, temp_layer.as_ref(), &build_cmds.format);
    let source_manifest = pull_result.manifest;
    let target_manifest = build_target_manifest(
        source_manifest, &build_cmds.format, temp_layer.as_ref())?;

    match &build_cmds.target {
        TargetType::Registry(image) => {
            let registry_adapter = RegistryTargetAdapter::new(
                image, build_cmds.format.clone(), !build_cmds.allow_insecure,
                target_manifest, target_config_blob, build_cmds.target_auth.as_ref())?;
            registry_adapter.upload()?
        }
    }

    Ok(())
}

/// 构建一个tar layer
fn build_top_tar(copyfiles: &[CopyFile], home_dir: &HomeDir) -> Result<Option<PathBuf>> {
    if copyfiles.is_empty() {
        return Ok(None);
    }
    let tar_file_name = random::random_str(10) + ".tar";
    let tar_temp_file_path = home_dir.cache.temp_dir.join(tar_file_name);
    let tar_temp_file = File::create(tar_temp_file_path.as_path())?;
    let mut tar_builder = Builder::new(tar_temp_file);
    for copyfile in copyfiles {
        for source_path_str in &copyfile.source_path {
            let source_path = PathBuf::from(&source_path_str);
            if !source_path.exists() {
                return Err(Error::msg(format!("path not found:{}", source_path_str)));
            }
            let dest_path = if copyfile.dest_path.ends_with('/') {
                &copyfile.dest_path[1..]
            } else { &copyfile.dest_path };
            if source_path.is_file() {
                tar_builder.append_file(dest_path, &mut File::open(source_path)?)?;
            } else if source_path.is_dir() {
                tar_builder.append_dir(dest_path, source_path_str)?;
            } else {
                return Err(Error::msg("copy only support file and dir".to_string()));
            }
        }
    }
    tar_builder.finish()?;
    Ok(Some(tar_temp_file_path))
}

/// 压缩tar layer文件为gz格式
fn gz_layer_file(tar_file_path: &Path, home_dir: &HomeDir) -> Result<TempLayerInfo> {
    let tar_file = File::open(tar_file_path)?;
    let mut sha256_reader = Sha256Reader::new(tar_file);
    let tgz_file_name = random::random_str(10) + ".tgz";
    let tgz_file_path = home_dir.cache.temp_dir.join(tgz_file_name);
    let tgz_file = File::create(&tgz_file_path)?;
    let mut sha256_writer = Sha256Writer::new(tgz_file);
    compress::gz_file(&mut sha256_reader, &mut sha256_writer)?;
    let tar_sha256 = sha256_reader.sha256()?;
    let tgz_sha256 = sha256_writer.sha256()?;
    Ok(TempLayerInfo {
        tgz_sha256,
        tar_sha256,
        compress_layer_path: tgz_file_path,
        compress_type: CompressType::Tgz,
    })
}

fn build_target_config_blob(
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

fn build_target_manifest(
    source_manifest: Manifest,
    target_format: &TargetFormat,
    temp_layer_opt: Option<&TempLayerInfo>,
) -> Result<Manifest> {
    let mut target_manifest = match target_format {
        TargetFormat::Docker => Manifest::DockerV2S2(source_manifest.to_docker_v2_s2()?),
        TargetFormat::Oci => Manifest::OciV1(source_manifest.to_oci_v1()?)
    };
    if let Some(temp_layer) = temp_layer_opt {
        let metadata = temp_layer.compress_layer_path.metadata()?;
        target_manifest.add_top_gz_layer(metadata.len(), temp_layer.tgz_sha256.to_string())
    }
    Ok(target_manifest)
}