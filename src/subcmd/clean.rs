use crate::config::cmd::CleanCmdArgs;
use crate::util::file::PathExt;
use crate::{HomeDir, GLOBAL_CONFIG};
use anyhow::Result;
use log::info;
use std::ops::Deref;

pub struct CleanCommand {}

impl CleanCommand {
    pub fn clean(clean_args: &CleanCmdArgs) -> Result<()> {
        match main_clean(clean_args) {
            Ok(_) => print_build_success(),
            Err(err) => print_build_failed(err),
        }
        Ok(())
    }
}

fn main_clean(clean_args: &CleanCmdArgs) -> Result<()> {
    let home_dir = GLOBAL_CONFIG.home_dir.clone();
    if clean_args.all {
        clean_temp(home_dir.deref())?;
        clean_blobs(home_dir.deref())?;
        clean_download(home_dir.deref())?;
        return Ok(());
    }
    if clean_args.temp {
        clean_temp(home_dir.deref())?
    }
    if clean_args.blob {
        clean_blobs(home_dir.deref())?
    }
    if clean_args.download {
        clean_download(home_dir.deref())?
    }
    Ok(())
}

fn clean_temp(home_dir: &HomeDir) -> Result<()> {
    let temp_path = &home_dir.cache.temp_dir;
    info!("Clean temps. (path={})", temp_path.to_string_lossy());
    temp_path.clean_path()
}

fn clean_blobs(home_dir: &HomeDir) -> Result<()> {
    let blob_path = &home_dir.cache.blobs.blob_path;
    info!("Clean blobs. (path={})", blob_path.to_string_lossy());
    blob_path.clean_path()
}

fn clean_download(home_dir: &HomeDir) -> Result<()> {
    let download_path = &home_dir.cache.blobs.download_dir;
    info!("Clean downloads. (path={})", download_path.to_string_lossy());
    download_path.clean_path()
}

fn print_build_success() {
    println!(
        r#"
Clean successful!
"#
    );
}

fn print_build_failed(err: anyhow::Error) {
    println!(
        r#"
Clean failed.
{}
"#,
        err
    );
}
