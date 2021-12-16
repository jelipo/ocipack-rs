use std::fs::File;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Error, Result};
use log::info;
use reqwest::Method;
use serde::de::DeserializeOwned;
use url::Url;

use manifest::Manifest;

use crate::reg::docker::DockerManifest;
use crate::reg::docker::image::DockerConfigBlob;
use crate::reg::home::HomeDir;
use crate::reg::http::auth::TokenType;
use crate::reg::http::client::{ClientRequest, RawRegistryResponse, RegistryHttpClient, RegistryResponse};
use crate::reg::http::download::RegDownloader;
use crate::reg::http::RegistryAuth;
use crate::reg::http::upload::RegUploader;
use crate::reg::manifest::CommonManifestLayer;
use crate::reg::oci::image::OciConfigBlob;
use crate::reg::oci::OciManifest;

pub mod home;
pub mod docker;
pub mod oci;
pub mod http;
pub mod manifest;


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

pub struct Registry {
    pub image_manager: MyImageManager,
}

impl Registry {
    pub fn open(
        registry_addr: String,
        auth: Option<RegistryAuth>,
        home_dir: Rc<HomeDir>,
    ) -> Result<Registry> {
        let client = RegistryHttpClient::new(registry_addr.clone(), auth)?;
        let image = MyImageManager::new(registry_addr.clone(), client, home_dir);
        Ok(Registry {
            image_manager: image,
        })
    }
}

pub struct MyImageManager {
    registry_addr: String,
    reg_client: RegistryHttpClient,
    home_dir: Rc<HomeDir>,
}

impl MyImageManager {
    pub fn new(
        registry_addr: String,
        client: RegistryHttpClient,
        home_dir: Rc<HomeDir>,
    ) -> MyImageManager {
        MyImageManager {
            registry_addr,
            reg_client: client,
            home_dir,
        }
    }

    /// 获取Image的Manifest
    pub fn manifests(&mut self, refe: &Reference) -> Result<Manifest> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name);
        let accepts = &[RegContentType::OCI_MANIFEST, RegContentType::DOCKER_MANIFEST];
        let request: ClientRequest<u8> = ClientRequest::new_get_request(&path, scope, accepts);
        let response = self.reg_client.simple_request(request)?;
        let content_type = (&response).content_type()
            .ok_or(Error::msg("manifest content-type header not found"))?;
        if RegContentType::DOCKER_MANIFEST.val() == content_type {
            let manifest = serde_json::from_str::<DockerManifest>(&response.string_body())?;
            Ok(Manifest::DockerV2S2(manifest))
        } else if RegContentType::OCI_MANIFEST.val() == content_type {
            let manifest = serde_json::from_str::<OciManifest>(&response.string_body())?;
            Ok(Manifest::OciV1(manifest))
        } else {
            let msg = format!("unknown content-type:{},body:{}", content_type.to_string(), response.string_body());
            Err(Error::msg(msg))
        }
    }

    /// Image manifests是否存在
    pub fn manifests_exited(&mut self, refe: &Reference) -> Result<bool> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name);
        let request = ClientRequest::new_head_request(&path, scope, TokenType::Pull);
        let response = self.reg_client.simple_request::<u8>(request)?;
        exited(&response)
    }

    /// Image blobs是否存在
    pub fn blobs_exited(&mut self, name: &str, blob_digest: &RegDigest) -> Result<bool> {
        let path = format!("/v2/{}/blobs/{}", name, blob_digest.digest);
        let scope = Some(name);
        let request = ClientRequest::new_head_request(&path, scope, TokenType::Pull);
        let response = self.reg_client.simple_request::<u8>(request)?;
        exited(&response)
    }

    pub fn config_blob<T: ConfigBlob + DeserializeOwned>(
        &mut self, name: &str, blob_digest: &str,
    ) -> Result<T> {
        let url_path = format!("/v2/{}/blobs/{}", name, blob_digest);
        let request: ClientRequest<u8> = ClientRequest::new_get_request(&url_path, Some(name), &[]);
        let response = self.reg_client.simple_request(request)?;
        Ok(serde_json::from_str::<T>(&response.string_body())?)
    }

    pub fn layer_blob_download(&mut self, name: &str, blob_digest: &RegDigest, layer_size: Option<u64>) -> Result<RegDownloader> {
        let url_path = format!("/v2/{}/blobs/{}", name, blob_digest.digest);
        let file_path = self.home_dir.cache.blobs.download_ready(blob_digest);
        let file_name = blob_digest.sha256.clone();
        let mut blob_config = BlobConfig::new(file_path, file_name, blob_digest.clone());
        if let Some(exists_file) = self.home_dir.cache.blobs.tgz_file_path(blob_digest) {
            let file = File::open(&exists_file)?;
            blob_config.file_path = exists_file;
            let finished_downloader = RegDownloader::new_finished_downloader(
                blob_config, file.metadata()?.len())?;
            return Ok(finished_downloader);
        }
        let downloader = self.reg_client.download(&url_path, blob_config, name, layer_size)?;
        Ok(downloader)
    }

    /// 上传layer类型的blob文件
    pub fn layer_blob_upload(&mut self, name: &str, blob_digest: &RegDigest, file_local_path: &str) -> Result<RegUploader> {
        let file_path = PathBuf::from(file_local_path).into_boxed_path();
        let file_name = file_path.file_name()
            .expect("file name error").to_str().unwrap().to_string();
        let blob_config = BlobConfig::new(file_path.clone(), file_name, blob_digest.clone());
        let short_hash = blob_config.short_hash.clone();
        if self.blobs_exited(name, &blob_digest)? {
            return Ok(RegUploader::new_finished_uploader(
                blob_config, file_path.metadata()?.len(),
                format!("{} blob exists in registry", short_hash),
            ));
        }
        let mut location_url = self.layer_blob_upload_ready(name)?;
        location_url.query_pairs_mut().append_pair("digest", &blob_digest.digest);
        let blob_upload_url = location_url.as_str();
        info!("blob_upload_url is {}",blob_upload_url);
        let reg_uploader = self.reg_client.upload(
            location_url.to_string(), blob_config, name, &file_path,
        )?;
        Ok(reg_uploader)
    }

    /// 向仓库获取上传blob的URL
    pub fn layer_blob_upload_ready(&mut self, name: &str) -> Result<Url> {
        let url_path = format!("/v2/{}/blobs/uploads/", name);
        let scope = Some(name);
        let request = ClientRequest::new(&url_path, scope, Method::POST, &[], None, TokenType::PushAndPull);
        let success_resp = self.reg_client.request_full_response::<u8>(request)?;
        let location = success_resp.location_header().expect("location header not found");
        let url = Url::parse(location)?;
        Ok(url)
    }

    pub fn put_manifest(&mut self, refe: &Reference, manifest: Manifest) -> Result<String> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name);
        let response = match manifest {
            Manifest::OciV1(oci_manifest) => {
                let request = ClientRequest::new_with_content_type(
                    &path, scope, Method::PUT, &[], Some(&oci_manifest),
                    &RegContentType::OCI_MANIFEST, TokenType::PushAndPull,
                );
                self.reg_client.simple_request::<OciManifest>(request)?
            }
            Manifest::DockerV2S2(docker_v2s2_manifest) => {
                let request = ClientRequest::new_with_content_type(
                    &path, scope, Method::PUT, &[], Some(&docker_v2s2_manifest),
                    &RegContentType::DOCKER_MANIFEST, TokenType::PushAndPull,
                );
                self.reg_client.simple_request::<DockerManifest>(request)?
            }
        };
        Ok(response.string_body())
    }
}

fn exited(simple_response: &RawRegistryResponse) -> Result<bool> {
    match simple_response.status_code() {
        200..300 => Ok(true),
        404 => Ok(false),
        status_code => {
            let msg = format!("request registry error,status code:{}", status_code);
            Err(Error::msg(msg))
        }
    }
}

pub trait LayerConvert {
    fn to_layers(&self) -> Vec<Layer>;
}

pub struct Layer<'a> {
    pub media_type: &'a str,
    pub size: u64,
    pub digest: &'a str,
}

pub trait ConfigBlob {}

#[derive(Clone)]
pub enum ConfigBlobEnum {
    OciV1(OciConfigBlob),
    DockerV2S2(DockerConfigBlob),
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

    pub const ALL: Self = Self("*/*");

    pub fn val(&self) -> &'static str {
        self.0
    }
}

