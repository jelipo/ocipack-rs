use std::cell::RefCell;
use std::fs::File;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Error, Result};
use log::info;
use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

use crate::reg::{BlobConfig, Layer, LayerConvert, Reference, RegDigest};
use crate::reg::home::HomeDir;
use crate::reg::http::auth::TokenType;
use crate::reg::http::client::{ClientRequest, RawRegistryResponse, RegistryHttpClient, RegistryResponse};
use crate::reg::http::download::RegDownloader;
use crate::reg::http::upload::RegUploader;
use crate::reg::oci::image::OciConfigBlob;
use crate::reg::RegContentType;

pub mod registry;
pub mod image;


pub struct OciImageManager {
    registry_addr: String,
    reg_client: Rc<RefCell<RegistryHttpClient>>,
    home_dir: Rc<HomeDir>,
}

impl OciImageManager {
    pub fn new(
        registry_addr: String,
        client: Rc<RefCell<RegistryHttpClient>>,
        home_dir: Rc<HomeDir>,
    ) -> OciImageManager {
        OciImageManager {
            registry_addr,
            reg_client: client,
            home_dir,
        }
    }

    /// 获取Image的Manifest
    pub fn manifests(&mut self, refe: &Reference) -> Result<OciManifest> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name);
        let mut reg_rc = self.reg_client.borrow_mut();
        let accepts = &[RegContentType::OCI_MANIFEST];
        let request = ClientRequest::new_get_request(&path, scope, accepts);
        reg_rc.request_registry_body::<u8, OciManifest>(request)
    }

    /// Image manifests是否存在
    pub fn manifests_exited(&mut self, refe: &Reference) -> Result<bool> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name);
        let request = ClientRequest::new_head_request(&path, scope, TokenType::Pull);
        let response = self.reg_client.borrow_mut().simple_request::<u8>(request)?;
        exited(&response)
    }

    /// Image blobs是否存在
    pub fn blobs_exited(&mut self, name: &str, blob_digest: &RegDigest) -> Result<bool> {
        let path = format!("/v2/{}/blobs/{}", name, blob_digest.digest);
        let scope = Some(name);
        let request = ClientRequest::new_head_request(&path, scope, TokenType::Pull);
        let response = self.reg_client.borrow_mut().simple_request::<u8>(request)?;
        exited(&response)
    }

    pub fn config_blob(&mut self, name: &str, blob_digest: &str) -> Result<OciConfigBlob> {
        let url_path = format!("/v2/{}/blobs/{}", name, blob_digest);
        let mut reg_rc = self.reg_client.borrow_mut();
        let request = ClientRequest::new_get_request(&url_path, Some(name), &[]);
        reg_rc.request_registry_body::<u8, OciConfigBlob>(request)
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
        let downloader = self.reg_client.borrow_mut().download(&url_path, blob_config, name, layer_size)?;
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
        let reg_uploader = self.reg_client.borrow_mut().upload(
            location_url.to_string(), blob_config, name, &file_path,
        )?;
        Ok(reg_uploader)
    }

    /// 向仓库获取上传blob的URL
    pub fn layer_blob_upload_ready(&mut self, name: &str) -> Result<Url> {
        let url_path = format!("/v2/{}/blobs/uploads/", name);
        let scope = Some(name);
        let mut reg_rc = self.reg_client.borrow_mut();
        let request = ClientRequest::new(&url_path, scope, Method::POST, &[], None, TokenType::PushAndPull);
        let success_resp = reg_rc.request_full_response::<u8>(request)?;
        let location = success_resp.location_header().expect("location header not found");
        let url = Url::parse(location)?;
        Ok(url)
    }

    pub fn put_manifest(&mut self, refe: &Reference, manifest: OciManifest) -> Result<String> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name);
        let mut reg_rc = self.reg_client.borrow_mut();
        let request = ClientRequest::new_with_content_type(
            &path, scope, Method::PUT, &[], Some(&manifest),
            &RegContentType::OCI_MANIFEST,
            TokenType::PushAndPull,
        );
        let raw_response = reg_rc.simple_request::<OciManifest>(request)?;
        Ok(raw_response.string_body())
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciManifest {
    pub schema_version: usize,
    pub media_type: String,
    pub config: OciManifestConfig,
    pub layers: Vec<OciManifestLayer>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciManifestConfig {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciManifestLayer {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}

impl LayerConvert for OciManifest {
    fn to_layers(&self) -> Vec<Layer> {
        self.layers.iter().map(|oci| Layer {
            media_type: &oci.media_type,
            size: oci.size,
            digest: &oci.digest,
        }).collect::<Vec<Layer>>()
    }
}
