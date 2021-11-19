use std::fs::File;
use std::path::Path;

use anyhow::Result;
use bytes::Bytes;
use sha2::{Digest, Sha256};
use sha2::digest::DynDigest;

pub fn sha256(bytes: &Bytes) -> String {
    let mut hasher = Sha256::new();
    DynDigest::update(&mut hasher, bytes.as_ref());
    let sha256 = &hasher.finalize()[..];
    hex::encode(sha256)
}

/// 计算文件的sha256值,并返回Hex
pub fn file_sha256(file_path: &Path) -> Result<String> {
    let mut file = File::open(file_path)?;
    let mut sha256 = Sha256::new();
    let _i = std::io::copy(&mut file, &mut sha256)?;
    let sha256 = &sha256.finalize()[..];
    Ok(hex::encode(sha256))
}

#[derive(Clone)]
pub struct Sha {
    pub sha_type: ShaType,
    pub sha_str: String,
}

#[derive(Clone)]
pub enum ShaType {
    Sha256,
    Sha128,
}
