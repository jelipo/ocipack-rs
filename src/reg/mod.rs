use std::detect::__is_feature_detected::sha;
use std::path::Path;

use anyhow::Result;
use url::Url;

use crate::reg::docker::registry::DockerRegistry;
use crate::reg::http::download::RegDownloader;
use crate::reg::http::upload::RegUploader;
use crate::reg::oci::registry::OciRegistry;

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

pub trait ImageManager {
    // /// 获取Image的Manifest
    // fn manifests(&mut self, refe: &Reference) -> Result<Manifest>;
    //
    // /// Image manifests是否存在
    // fn manifests_exited(&mut self, refe: &Reference) -> Result<bool>;
    //
    // /// Image blobs是否存在
    // fn blobs_exited(&mut self, name: &str, blob_digest: &RegDigest) -> Result<bool>;
    //
    // fn config_blob(&mut self, name: &str, blob_digest: &str) -> Result<ConfigBlob>;
    //
    // fn layer_blob_download(&mut self, name: &str, blob_digest: &RegDigest, layer_size: Option<u64>) -> Result<RegDownloader>;
    //
    // /// 上传layer类型的blob文件
    // fn layer_blob_upload(&mut self, name: &str, blob_digest: &RegDigest, file_local_path: &str) -> Result<RegUploader>;
    //
    // /// 向仓库获取上传blob的URL
    // fn layer_blob_upload_ready(&mut self, name: &str) -> Result<Url>;
    //
    // fn put_manifest(&mut self, refe: &Reference, manifest: Manifest2) -> Result<String>;
}

pub enum Registry {
    Docker(DockerRegistry),
    Oci(OciRegistry),
}

impl Registry {
    pub fn image_manager(&self) -> &dyn ImageManager {
        match self {
            Registry::Docker(docker_reg) => &docker_reg.docker_image_manager,
            Registry::Oci(oci_reg) => &oci_reg.oci_image_manager
        }
    }
}

pub struct RegContentType(&'static str);

impl RegContentType {
    /// Docker content-type
    pub const DOCKER_MANIFEST: Self = Self("application/vnd.docker.distribution.manifest.v2+json");
    pub const DOCKER_MANIFEST_LIST: Self = Self("application/vnd.docker.distribution.manifest.list.v2+json");
    pub const DOCKER_FOREIGN_LAYER_TGZ: Self = Self("application/vnd.docker.image.rootfs.foreign.diff.tar.gzip");
    pub const DOCKER_LAYER_TGZ: Self = Self("application/vnd.docker.image.rootfs.diff.tar.gzip");
    pub const DOCKER_CONTAINER_IMAGE: Self = Self("application/vnd.docker.container.image.v1+json");

    /// OCI content-type
    pub const OCI_MANIFEST: Self = Self("application/vnd.oci.image.manifest.v1+json");
    pub const OCI_LAYER_TAR: Self = Self("application/vnd.oci.image.layer.v1.tar");
    pub const OCI_LAYER_TGZ: Self = Self("application/vnd.oci.image.layer.v1.tar+gzip");
    pub const OCI_LAYER_NONDISTRIBUTABLE_TAR: Self = Self("application/vnd.oci.image.layer.nondistributable.v1.tar");
    pub const OCI_LAYER_NONDISTRIBUTABLE_TGZ: Self = Self("application/vnd.oci.image.layer.nondistributable.v1.tar+gzip");
    pub const OCI_IMAGE_CONFIG: Self = Self("application/vnd.oci.image.config.v1+json");

    pub const ALL: Self = Self(" */*");

    pub fn val(&self) -> &'static str {
        self.0
    }
}
