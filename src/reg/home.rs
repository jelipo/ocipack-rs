use std::path::Path;

use anyhow::Result;

use crate::BlobType;
use crate::util::sha::file_sha256;

pub struct HomeDir {
    pub cache: CacheDir,
}

impl HomeDir {
    pub fn new_home_dir(cache_dir_path: &Path) -> HomeDir {
        let blob_cache_dir_path = &cache_dir_path.join("blobs");
        HomeDir {
            cache: CacheDir {
                blobs: BlobsDir {
                    blob_path: blob_cache_dir_path.clone().into_boxed_path(),
                    config_path: blob_cache_dir_path.join("config").into_boxed_path(),
                    layers_path: blob_cache_dir_path.join("layers").into_boxed_path(),
                },
            },
        }
    }
}

pub struct CacheDir {
    pub blobs: BlobsDir,
}

pub struct BlobsDir {
    pub blob_path: Box<Path>,
    pub config_path: Box<Path>,
    pub layers_path: Box<Path>,
}

impl BlobsDir {
    pub fn digest_path(&self, digest: &str, blob_type: &BlobType) -> (Box<Path>, String) {
        let file_parent_dir = match blob_type {
            BlobType::Layers => self.blob_path.join("config"),
            BlobType::Config => self.blob_path.join("layers")
        };
        let clean_file_name = digest.replace(":", "_");
        let file_path = file_parent_dir.with_file_name(clean_file_name.as_str())
            .into_boxed_path();
        (file_path, clean_file_name)
    }

    /// 下载之前的前置检查
    /// 返回值bool代表是否需要下载
    pub fn download_pre_processing(&self, file_path: &Path, file_expect_sha256: String) -> Result<bool> {
        if !file_path.exists() {
            return Ok(true);
        }
        return if file_path.is_file() {
            let file_sha256 = file_sha256(file_path)?;
            if file_sha256 == file_expect_sha256 {
                Ok(false)
            } else {
                std::fs::remove_file(file_path)?;
                Ok(true)
            }
        } else {
            std::fs::remove_file(file_path)?;
            Ok(true)
        };
    }
}
