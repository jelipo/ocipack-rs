use std::cell::RefCell;
use std::fs::File;
use std::rc::Rc;

use anyhow::{Error, Result};
use log::{debug, info};
use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

use crate::reg::{BlobDownConfig, BlobType, Reference};
use crate::reg::docker::http::client::{RegistryHttpClient, RegistryResponse, SimpleRegistryResponse};
use crate::reg::docker::http::download::RegDownloader;
use crate::reg::docker::http::RegistryAccept;
use crate::reg::docker::image::ConfigBlob;
use crate::reg::home::HomeDir;

pub mod registry;
pub mod image;
pub mod http;


pub struct ImageManager {
    registry_addr: String,
    reg_client: Rc<RefCell<RegistryHttpClient>>,
    home_dir: Rc<HomeDir>,
}

impl ImageManager {
    pub fn new(
        registry_addr: String,
        client: Rc<RefCell<RegistryHttpClient>>,
        home_dir: Rc<HomeDir>,
    ) -> ImageManager {
        ImageManager {
            registry_addr,
            reg_client: client,
            home_dir,
        }
    }

    /// 获取Image的Manifest
    pub fn manifests(&mut self, refe: &Reference) -> Result<Manifest2> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name);
        let mut reg_rc = self.reg_client.borrow_mut();
        let accept_opt = Some(RegistryAccept::APPLICATION_VND_DOCKER_DISTRIBUTION_MANIFEST_V2JSON);
        reg_rc.request_registry_body::<u8, Manifest2>(&path, &scope, Method::GET, &accept_opt, None)
    }

    // /// 获取Image的Manifest
    // pub fn manifests_list(&mut self, refe: &Reference) -> Result<Manifest2> {
    //     let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
    //     let scope = Some(refe.image_name);
    //     let mut reg_rc = self.reg_client.borrow_mut();
    //     let accept_opt = Some(RegistryAccept::APPLICATION_VND_DOCKER_DISTRIBUTION_MANIFEST_LIST_V2JSON);
    //     let string = reg_rc.request_registry::<u8, String>(&path, &scope, Method::GET, &accept_opt, None)?;
    // }

    /// Image manifests是否存在
    pub fn manifests_exited(&mut self, refe: &Reference) -> Result<bool> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name);
        let response = self.reg_client.borrow_mut().head_request_registry(&path, &scope)?;
        exited(&response)
    }

    /// Image blobs是否存在
    pub fn blobs_exited(&mut self, name: &str, blob_digest: &str) -> Result<bool> {
        let path = format!("/v2/{}/blobs/{}", name, blob_digest);
        let scope = Some(name);
        let response = self.reg_client.borrow_mut().head_request_registry(&path, &scope)?;
        exited(&response)
    }

    pub fn config_blob(&mut self, name: &str, blob_digest: &str) -> Result<ConfigBlob> {
        let url_path = format!("/v2/{}/blobs/{}", name, blob_digest);
        let scope = Some(name);
        let mut reg_rc = self.reg_client.borrow_mut();
        reg_rc.request_registry_body::<u8, ConfigBlob>(&url_path, &scope, Method::GET, &None, None)
    }

    pub fn layer_blob_download(&mut self, name: &str, blob_digest: &str) -> Result<RegDownloader> {
        let url_path = format!("/v2/{}/blobs/{}", name, blob_digest);
        let (file_path, file_name) = self.home_dir.cache.blobs.digest_path(blob_digest, &BlobType::Layers);
        let down_config = BlobDownConfig {
            file_path,
            file_name,
            digest: blob_digest.to_string(),
            short_hash: blob_digest.replace("sha256:", "")[..12].to_string(),
        };
        let file_sha256 = blob_digest.replace("sha256:", "");
        if !self.home_dir.cache.blobs.download_pre_processing(&down_config.file_path, file_sha256)? {
            let file = File::open(&down_config.file_path)?;
            let finished_downloader = RegDownloader::new_finished_downloader(
                down_config, file.metadata()?.len() as usize)?;
            return Ok(finished_downloader);
        }
        let downloader = self.reg_client.borrow_mut().download(&url_path, down_config, name)?;
        Ok(downloader)
    }

    /// 上传layer类型的blob文件
    pub fn layer_blob_upload(&mut self, name: &str, blob_digest: &str, file_local_path: &str) -> Result<()> {
        if self.blobs_exited(name, blob_digest)? {
            // TODO 返回一个已经完成的结果
            return Ok(());
        }
        let mut location_url = self.layer_blob_upload_ready(name)?;
        location_url.query_pairs_mut().append_pair("digest", blob_digest);
        let blob_upload_url = location_url.as_str();
        info!("blob_upload_url is {}",blob_upload_url);

        Ok(())
    }

    /// 向仓库获取上传blob的URL
    fn layer_blob_upload_ready(&mut self, name: &str) -> Result<Url> {
        let url_path = format!("/v2/{}/blobs/uploads/", name);
        let scope = Some(name);
        let mut reg_rc = self.reg_client.borrow_mut();
        let success_resp = reg_rc.request_registry::<u8>(&url_path, &scope, Method::POST, &None, None)?;
        let location = success_resp.location_header().as_ref().expect("location header not found");
        let url = Url::parse(location)?;
        Ok(url)
    }
}

fn exited(simple_response: &SimpleRegistryResponse) -> Result<bool> {
    match simple_response.status_code() {
        200..300 => Ok(true),
        404 => Ok(false),
        status_code => {
            let msg = format!("request registry error,status code:{}", status_code);
            Err(Error::msg(msg))
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Manifest2 {
    pub schema_version: usize,
    pub media_type: String,
    pub config: ManifestConfig,
    pub layers: Vec<ManifestLayer>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ManifestConfig {
    pub media_type: String,
    pub size: usize,
    pub digest: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ManifestLayer {
    pub media_type: String,
    pub size: usize,
    pub digest: String,
    pub urls: Option<Vec<String>>,
}
