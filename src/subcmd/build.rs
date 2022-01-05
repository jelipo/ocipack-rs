use std::fs::File;
use std::path::{Path, PathBuf};

use anyhow::{Error, Result};
use tar::{Builder, Header};

use crate::{GLOBAL_CONFIG, HomeDir};
use crate::adapter::{BuildInfo, CopyFile, SourceImageAdapter, SourceInfo, TargetImageAdapter};
use crate::adapter::docker::DockerfileAdapter;
use crate::adapter::registry::RegistryTargetAdapter;
use crate::config::cmd::{BaseAuth, BuildCmdArgs, SourceType, TargetType};
use crate::config::RegAuthType;
use crate::reg::ConfigBlobEnum;
use crate::reg::home::TempLayerInfo;
use crate::subcmd::pull::pull;
use crate::tempconfig::TempConfig;
use crate::util::{compress, random};
use crate::util::sha::{Sha256Reader, Sha256Writer};

pub struct BuildCommand {}

impl BuildCommand {
    pub fn build(build_args: &BuildCmdArgs) -> Result<()> {
        let (source_info, build_info, source_auth) = build_source_info(build_args)?;

        Ok(())
    }
}

fn build_source_info(build_args: &BuildCmdArgs) -> Result<(SourceInfo, BuildInfo, RegAuthType)> {
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
    build_cmds: &BuildCmdArgs,
) -> Result<()> {
    let home_dir = GLOBAL_CONFIG.home_dir.clone();
    let pull = pull(&source_info, source_auth, !build_cmds.allow_insecure)?;

    let temp_layer = build_top_tar(&build_info.copy_files, &home_dir)?.map(|tar_path| {
        gz_layer_file(&tar_path, &home_dir)?
    });

    match &build_cmds.target {
        TargetType::Registry(image) => {
            let registry_adapter = RegistryTargetAdapter::new(
                image, build_cmds.format.clone(), !build_cmds.allow_insecure)?;
        }
    }


    Ok(())
}

fn build_top_tar(copyfiles: &Vec<CopyFile>, home_dir: &HomeDir) -> Result<Option<PathBuf>> {
    if copyfiles.len() == 0 {
        return Ok(None);
    }
    let tar_file_name = random::random_str(10) + ".tar";
    let tar_temp_file_path = home_dir.cache.temp_dir.join(tar_file_name);
    let tar_temp_file = File::create(tar_temp_file_path.as_path())?;
    let mut tar_builder = Builder::new(tar_temp_file);
    for copyfile in copyfiles {
        for source_path_str in &copyfile.source_path {
            let source_pathbuf = PathBuf::from(&source_path_str);
            if !source_pathbuf.exists() {
                return Err(Error::msg(format!("path not found:{}", source_path_str)));
            }
            let dest_path = if copyfile.dest_path.ends_with("/") {
                &copyfile.dest_path[1..]
            } else {
                &copyfile.dest_path
            };
            if source_pathbuf.is_file() {
                tar_builder.append_file(dest_path, &mut File::open(source_pathbuf)?)?;
            } else if source_pathbuf.is_dir() {
                tar_builder.append_dir(dest_path, source_path_str)?;
            } else {
                return Err(Error::msg(format!("copy only support file and dir")));
            }
        }
    }
    tar_builder.finish()?;
    Ok(Some(tar_temp_file_path))
}

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
    Ok(LayerInfo {
        gz_sha256: tgz_sha256,
        tar_sha256,
        gz_temp_file_path: tgz_file_path.into_boxed_path(),
    })
}

fn build_config_blob(
    build_info: BuildInfo,
    source_config_blob: &ConfigBlobEnum,
    temp_layer: &TempLayerInfo
) {
    let target_config_blob = source_config_blob.clone();
    let config_blob_digest = format!("sha256:{}", temp_layer.tar_sha256);
    let config_blob_str = match target_config_blob {
        ConfigBlobEnum::OciV1(mut oci_config_blob) => {
            oci_config_blob.rootfs.diff_ids.insert(0, config_blob_digest);
            serde_json::to_string(&oci_config_blob)?
        }
        ConfigBlobEnum::DockerV2S2(mut docker_config_blob) => {
            docker_config_blob.rootfs.diff_ids.insert(0, config_blob_digest);
            serde_json::to_string(&docker_config_blob)?
        }
    };
}
