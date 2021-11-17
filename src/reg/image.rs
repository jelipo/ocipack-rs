use std::cell::RefCell;
use std::fs::File;

use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::{Error, Result};
use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;

use crate::reg::home::HomeDir;
use crate::reg::http::client::{RegistryHttpClient, RegistryResponse, SimpleRegistryResponse};
use crate::reg::http::download::RegDownloader;
use crate::reg::http::RegistryAccept;
use crate::reg::{BlobType, Reference};
use crate::util::sha::file_sha256;

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
        let blobs_cache_path = match blob_type {
            BlobType::Layers => self.home_dir.cache.blobs.layers_path.clone(),
            BlobType::Config => self.home_dir.cache.blobs.config_path.clone()
        };
        let file_name = digest.replace(":", "_");
        let file_path = blobs_cache_path.join(file_name.as_str());
        let file_path_string = file_path.to_str().unwrap().to_string();
        let file_path_arc = Arc::new(file_path_string);
        if !self.download_check(file_name.as_str(), file_path.as_path())? {
            let file = File::open(file_path)?;
            let finished_downloader = RegDownloader::new_finished_downloader(
                file_path_arc.clone(), file.metadata()?.len() as usize)?;
            return Ok(finished_downloader);
        }
        let downloader = self.reg_client.borrow_mut().download(&url_path, file_path_arc.clone(), name)?;
        Ok(downloader)
    }

    /// 下载前置检查，是否需要下载
    fn download_check(&self, file_name: &str, file_path: &Path) -> Result<bool> {
        return if file_path.exists() {
            if file_name.starts_with("sha256_") {
                let file_sha256 = file_sha256(file_path)?;
                let need_sha256 = file_name.replace("sha256_", "");
                if file_sha256 == need_sha256 {
                    Ok(false)
                } else {
                    std::fs::remove_file(file_path)?;
                    Ok(true)
                }
            } else {
                std::fs::remove_file(file_path)?;
                Ok(true)
            }
        } else {
            Ok(true)
        };
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
