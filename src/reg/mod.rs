use std::detect::__is_feature_detected::sha;
use std::path::Path;

pub mod home;
pub mod docker;
pub mod oci;
pub mod http;


pub struct Reference<'a> {
    /// Image的名称
    pub image_name: &'a str,
    /// 可以是TAG或者digest
    pub reference: &'a str,
}

pub enum BlobType {
    Layers,
    Config,
}

pub struct BlobConfig {
    pub file_path: Box<Path>,
    pub file_name: String,
    pub reg_digest: RegDigest,
    pub short_hash: String,
}

impl BlobConfig {
    pub fn new(file_path: Box<Path>, file_name: String, digest: RegDigest) -> BlobConfig {
        BlobConfig {
            file_path,
            file_name,
            short_hash: digest.sha256[..12].to_string(),
            reg_digest: digest,
        }
    }
}

#[derive(Clone)]
pub struct RegDigest {
    pub sha256: String,
    pub digest: String,
}

impl RegDigest {
    pub fn new_with_sha256(sha256: String) -> RegDigest {
        RegDigest {
            digest: format!("sha256:{}", &sha256),
            sha256,
        }
    }

    pub fn new_with_digest(digest: String) -> RegDigest {
        RegDigest {
            sha256: digest.as_str()[7..].to_string(),
            digest,
        }
    }
}
