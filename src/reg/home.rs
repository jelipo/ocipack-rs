use std::fs::{create_dir, create_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};
use sha2::digest::DynDigest;

use crate::BlobType;
use crate::util::compress::ungzip_file;
use crate::util::file::PathExt;
use crate::util::sha::{file_sha256, sha256};

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
                    download_path: cache_dir_path.join("download").into_boxed_path(),
                },
                temp_dir: cache_dir_path.join("temp").into_boxed_path(),
            },
        };
        create_dir_all(&home_dir.cache.temp_dir)?;
        create_dir_all(&home_dir.cache.blobs.config_path)?;
        create_dir_all(&home_dir.cache.blobs.layers_path)?;
        create_dir_all(&home_dir.cache.blobs.layers_path)?;
        create_dir_all(&home_dir.cache.blobs.download_path)?;
        Ok(home_dir)
    }
}

pub struct CacheDir {
    pub blobs: BlobsDir,
    pub temp_dir: Box<Path>,
}

pub struct BlobsDir {
    blob_path: Box<Path>,
    pub config_path: Box<Path>,
    pub layers_path: Box<Path>,
    pub download_path: Box<Path>,
}

impl BlobsDir {
    pub fn download_ready(&self, digest: &str) -> (Box<Path>, String) {
        let file_parent_dir = &self.download_path;
        let download_file_sha256 = digest.replace("sha256:", "");
        let file_path = file_parent_dir.join(&download_file_sha256)
            .into_boxed_path();
        (file_path, download_file_sha256)
    }

    pub fn ungzip_download_file(&self, digest: &str) -> Result<Box<Path>> {
        let (download_file_path, download_file_sha256) = self.download_ready(digest);
        let download_file = File::open(&download_file_path)?;
        let mut sha256_encode = Sha256::new();
        ungzip_file(&download_file, &mut sha256_encode)?;
        drop(download_file);
        let sha256 = &sha256_encode.finalize()[..];
        let ungzip_sha256 = hex::encode(sha256);
        let layer_dir = self.layers_path.join(download_file_sha256);
        let ungzip_file_path = layer_dir.join(&ungzip_sha256);
        ungzip_file_path.remove()?;
        create_dir_all(&layer_dir)?;
        std::fs::rename(download_file_path, &ungzip_file_path)?;
        let ungizip_sha_file_path = self.ungizip_sha_file_path(&layer_dir);
        ungizip_sha_file_path.remove()?;
        let mut ungizip_sha_file = File::create(ungizip_sha_file_path)?;
        ungizip_sha_file.write(ungzip_sha256.as_bytes())?;
        ungizip_sha_file.flush()?;
        Ok(ungzip_file_path.into_boxed_path())
    }

    pub fn ungizip_sha_file_path(&self, layer_dir: &Path) -> Box<Path> {
        layer_dir.join("ungzip_sha256").into_boxed_path()
    }

    pub fn ungizip_file_path(&self, digest: &str) -> Option<Box<Path>> {
        let gzip_file_sha256 = self.digest_to_sha(digest);
        let layer_dir = self.layers_path.join(gzip_file_sha256);
        let ungzip_sha_file = self.ungizip_sha_file_path(layer_dir.as_path());
        if let Ok(mut file) = File::open(ungzip_sha_file) {
            let mut ungzip_file_sha256 = String::new();
            file.read_to_string(&mut ungzip_file_sha256);
            return Some(layer_dir.join(ungzip_file_sha256).into_boxed_path());
        }
        return None;
    }

    fn digest_to_sha(&self, digest: &str) -> String {
        digest.replace("sha256:", "")
    }

    pub fn layer_exists(&self, digest: &str) {}

    // /// 下载之前的前置检查
    // /// 返回值bool代表是否需要下载
    // pub fn download_pre_processing(&self, file_path: &Path, file_expect_sha256: &str) -> Result<bool> {
    //     if !file_path.exists() {
    //         return Ok(true);
    //     }
    //     return if file_path.is_file() {
    //         let file_sha256 = file_sha256(file_path)?;
    //         if file_sha256 == file_expect_sha256 {
    //             Ok(false)
    //         } else {
    //             std::fs::remove_file(file_path)?;
    //             Ok(true)
    //         }
    //     } else {
    //         std::fs::remove_file(file_path)?;
    //         Ok(true)
    //     };
    // }
}
