use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::{compress, random};
use crate::reg::RegDigest;
use crate::util::compress::ungz_file;
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

impl CacheDir {
    pub fn gz_layer_file(&self, tar_file_path: &Path) -> Result<LayerInfo> {
        let tar_file = File::open(tar_file_path)?;
        let mut sha256_reader = Sha256Reader::new(tar_file);
        let tgz_file_name = random::random_str(10) + ".tgz";
        let tgz_file_path = self.temp_dir.join(tgz_file_name);
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
    blob_path: Box<Path>,
    pub config_path: Box<Path>,
    pub layers_path: Box<Path>,
    pub download_path: Box<Path>,
}

impl BlobsDir {
    pub fn download_ready(&self, digest: &RegDigest) -> Box<Path> {
        let file_parent_dir = &self.download_path;
        let file_path = file_parent_dir.join(&digest.sha256)
            .into_boxed_path();
        file_path
    }

    pub fn ungz_download_file(&self, digest: &RegDigest) -> Result<Box<Path>> {
        let download_file_path = self.download_ready(digest);
        let download_file = File::open(&download_file_path)?;
        let mut sha256_encode = Sha256::new();
        ungz_file(&download_file, &mut sha256_encode)?;
        drop(download_file);
        let sha256 = &sha256_encode.finalize()[..];
        let tar_sha256 = hex::encode(sha256);
        let layer_dir = self.layers_path.join(&digest.sha256);
        let tar_file_path = layer_dir.join(&tar_sha256);
        tar_file_path.remove()?;
        create_dir_all(&layer_dir)?;
        std::fs::rename(download_file_path, &tar_file_path)?;
        let ungizip_sha_file_path = self.ungzip_sha_file_path(&layer_dir);
        ungizip_sha_file_path.remove()?;
        let mut tar_sha_file = File::create(ungizip_sha_file_path)?;
        tar_sha_file.write(tar_sha256.as_bytes())?;
        tar_sha_file.flush()?;
        Ok(tar_file_path.into_boxed_path())
    }

    pub fn ungzip_sha_file_path(&self, layer_dir: &Path) -> Box<Path> {
        layer_dir.join("tar_sha256").into_boxed_path()
    }


    pub fn tgz_file_path(&self, digest: &RegDigest) -> Option<Box<Path>> {
        let layer_file_parent = self.layers_path.join(&digest.sha256);
        let ungzip_sha_file = self.ungzip_sha_file_path(layer_file_parent.as_path());
        if let Ok(mut file) = File::open(ungzip_sha_file) {
            let mut tgz_file_name = String::new();
            file.read_to_string(&mut tgz_file_name);
            return Some(layer_file_parent.join(tgz_file_name).into_boxed_path());
        }
        return None;
    }

    fn digest_to_sha(&self, digest: &str) -> String {
        digest.replace("sha256:", "")
    }
}

pub struct LayerInfo {
    pub gz_sha256: String,
    pub tar_sha256: String,
    pub gz_temp_file_path: Box<Path>,
}