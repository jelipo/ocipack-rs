use std::path::Path;

pub mod home;
pub mod docker;


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
    pub digest: String,
    pub short_hash: String,
    pub sha256: String,
}

impl BlobConfig {
    pub fn new(file_path: Box<Path>, file_name: String, digest: String) -> BlobConfig {
        let sha256 = digest.replace("sha256:", "");
        BlobConfig {
            file_path,
            file_name,
            digest: digest.to_string(),
            short_hash: sha256[..12].to_string(),
            sha256,
        }
    }
}