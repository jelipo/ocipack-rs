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

pub struct BlobDownConfig {
    pub file_path: Box<Path>,
    pub file_name: String,
    pub digest: String,
    pub short_hash: String,
    pub blob_type: BlobType,
}