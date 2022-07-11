use std::fs;
use std::fs::{create_dir_all, read_to_string, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{anyhow, Result};

use crate::reg::{CompressType, RegDigest};
use crate::util::random;

pub struct HomeDir {
    pub cache: CacheDir,
}

impl HomeDir {
    pub fn new_home_dir(cache_dir_path: &Path) -> Result<HomeDir> {
        let blob_cache_dir_path = &cache_dir_path.join("blobs");
        let home_dir = HomeDir {
            cache: CacheDir {
                blobs: BlobsDir {
                    _blob_path: blob_cache_dir_path.clone().into_boxed_path(),
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
    pub fn write_temp_file(&self, file_str: String) -> Result<Box<Path>> {
        let temp_file_name = random::random_str(10) + ".tmp";
        let temp_file_path = self.temp_dir.join(temp_file_name);
        let mut temp_file = File::create(&temp_file_path)?;
        temp_file.write_all(file_str.as_bytes())?;
        temp_file.flush()?;
        Ok(temp_file_path.into_boxed_path())
    }
}

pub struct BlobsDir {
    _blob_path: Box<Path>,
    pub config_path: Box<Path>,
    pub layers_path: Box<Path>,
    pub download_dir: Box<Path>,
}

impl BlobsDir {
    pub fn download_ready(&self, digest: &RegDigest) -> Box<Path> {
        let file_parent_dir = &self.download_dir;
        file_parent_dir.join(&digest.sha256).into_boxed_path()
    }

    pub fn create_layer_config(
        &self,
        diff_layer_sha256: &str,
        manifest_layer_sha: &str,
        compress_type: CompressType,
    ) -> Result<LocalLayer> {
        let local_layer = LocalLayer::new(
            diff_layer_sha256.to_string(),
            manifest_layer_sha.to_string(),
            compress_type,
            &self.layers_path,
        );
        local_layer.update_local_config()?;
        Ok(local_layer)
    }

    pub fn diff_layer_config_path(&self, layer_dir: &Path) -> PathBuf {
        layer_dir.join("diff_layer")
    }

    /// Find layer in local
    pub fn local_layer(&self, manifest_layer_digest: &RegDigest) -> Option<LocalLayer> {
        match LocalLayer::try_pares(&self.layers_path, &manifest_layer_digest.sha256) {
            Ok(local) => Some(local),
            Err(_) => None,
        }
    }

    pub fn move_to_blob(&self, file_path: &Path, manifest_sha: &str, diff_layer_sha: &str) -> Result<()> {
        let diff_layer_dir = self.layers_path.join(manifest_sha);
        let diff_layer = diff_layer_dir.join(diff_layer_sha);
        if diff_layer.exists() {
            fs::remove_file(&diff_layer)?;
        }
        let diff_layer_parent = diff_layer.parent().ok_or_else(|| anyhow!("diff_layer must have a parent dir"))?;
        create_dir_all(diff_layer_parent)?;
        fs::rename(file_path, diff_layer)?;
        Ok(())
    }
}

pub struct TempLayerInfo {
    pub compressed_tar_sha256: String,
    pub tar_sha256: String,
    pub compress_layer_path: PathBuf,
    pub compress_type: CompressType,
}

pub struct LocalLayer {
    pub manifest_sha: String,
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
        let config_str = read_to_string(diff_layer_config_path.clone())?;
        let (compress_type, diff_layer_sha) = LocalLayer::pares_config(&config_str)?;
        let layer_file_path = diff_layer_dir.join(diff_layer_sha);
        if !layer_file_path.exists() {
            return Err(anyhow!("diff layer not found"));
        }
        Ok(LocalLayer {
            manifest_sha: manifest_sha.to_string(),
            diff_layer_sha: diff_layer_sha.to_string(),
            compress_type,
            diff_layer_config_path,
            layer_file_path,
        })
    }

    fn pares_config(config_str: &str) -> Result<(CompressType, &str)> {
        let split = config_str.split('\n').collect::<Vec<&str>>();
        if split.len() < 2 {
            return Err(anyhow!("error diff layer config file"));
        }
        let compress_type = CompressType::from_str(split[0])?;
        let diff_layer_sha = split[1];
        Ok((compress_type, diff_layer_sha))
    }

    pub fn layer_path(&self) -> String {
        self.layer_file_path.to_string_lossy().to_string()
    }

    pub fn new(diff_layer_sha: String, manifest_sha: String, compress_type: CompressType, layer_cache_dir: &Path) -> LocalLayer {
        let diff_layer_dir = layer_cache_dir.join(&manifest_sha);
        let config_name = Self::diff_layer_config_name();
        let diff_layer_config_path = diff_layer_dir.join(config_name);
        let layer_file_path = diff_layer_dir.join(&diff_layer_sha);
        LocalLayer {
            manifest_sha,
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
            fs::remove_file(&self.diff_layer_config_path)?;
        }
        let config_data = self.config_string();
        let parent = self.diff_layer_config_path.parent().unwrap();
        create_dir_all(parent)?;
        let mut diff_layer_config = File::create(&self.diff_layer_config_path)?;
        diff_layer_config.write_all(config_data.as_bytes())?;
        diff_layer_config.flush()?;
        Ok(())
    }

    pub fn diff_layer_config_name() -> &'static str {
        "diff_layer_config"
    }
}
