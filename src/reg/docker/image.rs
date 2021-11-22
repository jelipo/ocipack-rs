use std::cell::RefCell;
use std::fs::File;


use std::rc::Rc;


use anyhow::{Error, Result};
use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;

use crate::reg::home::HomeDir;
use crate::reg::docker::http::client::{RegistryHttpClient, RegistryResponse, SimpleRegistryResponse};
use crate::reg::docker::http::download::RegDownloader;
use crate::reg::docker::http::RegistryAccept;
use crate::reg::{BlobDownConfig, BlobType, Reference};


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
        reg_rc.request_registry::<u8, Manifest2>(&path, &scope, Method::GET, &accept_opt, None)
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
    pub fn blobs_exited(&mut self, name: &str, digest: &str) -> Result<bool> {
        let path = format!("/v2/{}/blobs/{}", name, digest);
        let scope = Some(name);
        let response = self.reg_client.borrow_mut().head_request_registry(&path, &scope)?;
        exited(&response)
    }

    pub fn blobs_download(&mut self, name: &str, digest: &str, blob_type: BlobType) -> Result<RegDownloader> {
        let url_path = format!("/v2/{}/blobs/{}", name, digest);
        let (file_path, file_name) = self.home_dir.cache.blobs.digest_path(digest, &blob_type);
        let down_config = BlobDownConfig {
            file_path,
            file_name,
            digest: digest.to_string(),
            short_hash: digest.replace("sha256:", "")[..12].to_string(),
            blob_type,
        };
        let file_sha256 = digest.replace("sha256:", "");
        if !self.home_dir.cache.blobs.download_pre_processing(&down_config.file_path, file_sha256)? {
            let file = File::open(&down_config.file_path)?;
            let finished_downloader = RegDownloader::new_finished_downloader(
                down_config, file.metadata()?.len() as usize)?;
            return Ok(finished_downloader);
        }
        let downloader = self.reg_client.borrow_mut().download(&url_path, down_config, name)?;
        Ok(downloader)
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
    media_type: String,
    size: usize,
    digest: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ManifestLayer {
    pub media_type: String,
    pub size: usize,
    pub digest: String,
    pub urls: Option<Vec<String>>,
}
