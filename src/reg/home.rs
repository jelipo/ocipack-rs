use std::fmt::format;
use std::fs::{create_dir_all, File, read_to_string};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Error, Result};
use sha2::{Digest, Sha256};

use crate::reg::{CompressType, RegDigest};
use crate::util::{compress, random};
use crate::util::compress::ungz;
use crate::util::file::PathExt;
use crate::util::sha::{Sha256Reader, Sha256Writer};

pub struct HomeDir {
    pub cache: CacheDir,
}

impl HomeDir {
    pub fn new_home_dir(cache_dir_path: &Path) -> Result<HomeDir> {
        let blob_cache_dir_path = &cache_dir_path.join("blobs");
        let home_dir = HomeDir {
            cache: CacheDir {
                blobs: BlobsDir {
                    blob_path: blob_cache_dir_path.clone().into_boxed_path(),
                    config_path: blob_cache_dir_path.join("config").into_boxed_path(),
                    layers_path: blob_cache_dir_path.join("layers").into_boxed_path(),
                    download_dir: cache_dir_path.join("download").into_boxed_path(),
                },
                temp_dir: cache_dir_path.join("temp").into_boxed_path(),
            },
        };
        create_dir_all(&home_dir.cache.temp_dir)?;
        create_dir_all(&home_dir.cache.blobs.config_path)?;
        create_dir_all(&home_dir.cache.blobs.layers_path)?;
        create_dir_all(&home_dir.cache.blobs.layers_path)?;
        create_dir_all(&home_dir.cache.blobs.download_dir)?;
        Ok(home_dir)
    }
}

pub struct CacheDir {
    pub blobs: BlobsDir,
    pub temp_dir: Box<Path>,
}

impl CacheDir {
    pub fn gz_layer_file(&self, tar_file_path: &Path) -> Result<TempLayerInfo> {
        let tar_file = File::open(tar_file_path)?;
        let mut sha256_reader = Sha256Reader::new(tar_file);
        let tgz_file_name = random::random_str(10) + ".tgz";
        let tgz_file_path = self.temp_dir.join(tgz_file_name);
        let tgz_file = File::create(&tgz_file_path)?;
        let mut sha256_writer = Sha256Writer::new(tgz_file);
        compress::gz_file(&mut sha256_reader, &mut sha256_writer)?;
        let tar_sha256 = sha256_reader.sha256()?;
        let tgz_sha256 = sha256_writer.sha256()?;
        Ok(TempLayerInfo {
            tgz_sha256,
            tar_sha256,
            gz_temp_file_path: tgz_file_path.into_boxed_path(),
        })
    }

    pub fn write_temp_file(&self, file_str: String) -> Result<Box<Path>> {
        let temp_file_name = random::random_str(10) + ".tmp";
        let temp_file_path = self.temp_dir.join(temp_file_name);
        let mut temp_file = File::create(&temp_file_path)?;
        temp_file.write_all(file_str.as_bytes())?;
        temp_file.flush()?;
        Ok(temp_file_path.into_boxed_path())
    }

    pub fn move_temp_to_blob(&self, file_name: &str, manifest_sha: &str, diff_layer_sha: &str) {
        let temp_file = self.temp_dir.join(file_name);
        // TODO
    }
}


pub struct BlobsDir {
    blob_path: Box<Path>,
    pub config_path: Box<Path>,
    pub layers_path: Box<Path>,
    pub download_dir: Box<Path>,
}

impl BlobsDir {
    pub fn download_ready(&self, digest: &RegDigest) -> Box<Path> {
        let file_parent_dir = &self.download_dir;
        let file_path = file_parent_dir.join(&digest.sha256)
            .into_boxed_path();
        file_path
    }

    pub fn save_layer_cache(
        &self, digest: &RegDigest, compress_type: CompressType,
    ) -> Result<()> {
        let download_path = self.download_dir.join(&digest.sha256);
        let _download_file = File::open(&download_path)?;
        match compress_type {
            CompressType::TAR => {}
            CompressType::TGZ => {}
            CompressType::ZSTD => {}
        }
        Ok(())
    }

    pub fn ungz_download_file(&self, digest: &RegDigest) -> Result<(String, Box<Path>)> {
        let tgz_download_file_path = self.download_ready(digest);
        let download_file = File::open(&tgz_download_file_path)?;
        let mut sha256_encode = Sha256::new();
        ungz(download_file, &mut sha256_encode)?;
        let sha256 = &sha256_encode.finalize()[..];
        let tar_sha256 = hex::encode(sha256);
        let layer_dir = self.layers_path.join(&digest.sha256);
        let tar_file_path = layer_dir.join(&tar_sha256);
        tar_file_path.remove()?;
        create_dir_all(&layer_dir)?;
        std::fs::rename(tgz_download_file_path, &tar_file_path)?;
        Ok((tar_sha256, tar_file_path.into_boxed_path()))
    }

    pub fn create_layer_config(
        &self,
        tar_sha256: &str,
        manifest_layer_sha: &str,
        compress_type: CompressType,
    ) -> Result<()> {
        // TODO use LocalLayer
        let local_layer = LocalLayer::new(
            tar_sha256.to_string(), manifest_layer_sha.to_string(),
            compress_type, &self.layers_path);
        local_layer.update_local_config()
    }

    pub fn diff_layer_config_path(&self, layer_dir: &Path) -> PathBuf {
        layer_dir.join("diff_layer")
    }

    // pub fn diff_layer_path(&self, digest: &RegDigest) -> Option<PathBuf> {
    //     let layer_file_parent = self.layers_path.join(&digest.sha256);
    //     let diff_layer_config = self.diff_layer_config_path(layer_file_parent.as_path());
    //     match read_to_string(diff_layer_config) {
    //         Ok(tgz_file_name) => Some(layer_file_parent.join(tgz_file_name)),
    //         Err(_) => None
    //     }
    // }

    /// Find layer in local
    pub fn local_layer(&self, manifest_layer_digest: &RegDigest) -> Option<LocalLayer> {
        match LocalLayer::try_pares(&self.layers_path, &manifest_layer_digest.sha256) {
            Ok(local) => Some(local),
            Err(_) => None,
        }
    }
}

pub struct TempLayerInfo {
    pub tgz_sha256: String,
    pub tar_sha256: String,
    pub gz_temp_file_path: Box<Path>,
}


pub struct LocalLayer {
    pub diff_layer_sha: String,
    pub compress_type: CompressType,
    pub diff_layer_config_path: PathBuf,
    pub layer_file_path: PathBuf,
}

impl LocalLayer {
    pub fn try_pares(layer_cache_dir: &Path, manifest_sha: &str) -> Result<LocalLayer> {
        let diff_layer_dir = layer_cache_dir.join(manifest_sha);
        let config_name = Self::diff_layer_config_name();
        let diff_layer_config_path = diff_layer_dir.join(config_name);
        let config_str = read_to_string(diff_layer_dir.clone())?;
        let (compress_type, diff_layer_sha) = LocalLayer::pares_config(&config_str)?;
        let layer_file_path = diff_layer_dir.join(diff_layer_sha);
        if !layer_file_path.exists() { return Err(Error::msg("diff layer not found")); }
        Ok(LocalLayer {
            diff_layer_sha: diff_layer_sha.to_string(),
            compress_type,
            diff_layer_config_path,
            layer_file_path,
        })
    }

    fn pares_config(config_str: &str) -> Result<(CompressType, &str)> {
        let split = config_str.split('\n').collect::<Vec<&str>>();
        if split.len() < 2 { return Err(Error::msg("error diff layer config file")); }
        let compress_type = CompressType::from_str(split[0])?;
        let diff_layer_sha = split[2];
        Ok((compress_type, diff_layer_sha))
    }

    pub fn layer_path(&self) -> String {
        self.layer_file_path.to_string_lossy().to_string()
    }

    pub fn new(
        diff_layer_sha: String,
        manifest_sha: String,
        compress_type: CompressType,
        layer_cache_dir: &Path,
    ) -> LocalLayer {
        let diff_layer_dir = layer_cache_dir.join(manifest_sha);
        let config_name = Self::diff_layer_config_name();
        let diff_layer_config_path = diff_layer_dir.join(config_name);
        let layer_file_path = diff_layer_dir.join(&diff_layer_sha);
        LocalLayer {
            diff_layer_sha,
            compress_type,
            diff_layer_config_path,
            layer_file_path,
        }
    }

    pub fn config_string(&self) -> String {
        let compress_type = self.compress_type.to_string();
        format!("{}\n{}", compress_type, &self.diff_layer_sha)
    }

    pub fn update_local_config(&self) -> Result<()> {
        if self.diff_layer_config_path.exists() {
            std::fs::remove_file(&self.diff_layer_config_path)?;
        }
        let config_data = self.config_string();
        let mut diff_layer_config = File::create(&self.diff_layer_config_path)?;
        diff_layer_config.write(config_data.as_bytes())?;
        diff_layer_config.flush()?;
        Ok(())
    }

    pub fn diff_layer_config_name() -> &'static str {
        "diff_layer_config"
    }
}