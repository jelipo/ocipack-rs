use std::fs::File;
use std::path::Path;
use bytes::Bytes;
use sha2::digest::DynDigest;
use sha2::{Digest, Sha256};
use anyhow::Result;

pub fn sha256(bytes: &Bytes) -> String {
    let mut hasher = Sha256::new();
    DynDigest::update(&mut hasher, bytes.as_ref());
    let sha256 = &hasher.finalize()[..];
    hex::encode(sha256)
}

pub fn file_sha256(file_path: &Path) -> Result<String> {
    let mut file = File::open(file_path)?;
    let mut sha256 = Sha256::new();
    let i = std::io::copy(&mut file, &mut sha256)?;
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