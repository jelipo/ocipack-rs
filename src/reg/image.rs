use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{Error, Result};
use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;

use crate::reg::home::HomeDir;
use crate::reg::http::client::{RegistryHttpClient, RegistryResponse, SimpleRegistryResponse};
use crate::reg::http::download::{CustomDownloadFileName, DownloadFilenameType, RegDownloader};
use crate::reg::Reference;
use crate::util::sha::{file_sha256, Sha, sha256, ShaType};

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
        let scope = Some(refe.image_name.to_string());
        let mut reg_rc = self.reg_client.borrow_mut();
        reg_rc.request_registry::<u8, Manifest2>(&path, &scope, Method::GET, None)
    }

    /// Image manifests是否存在
    pub fn manifests_exited(&mut self, refe: &Reference) -> Result<bool> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name.to_string());
        let response = self.reg_client.borrow_mut().head_request_registry(&path, &scope)?;
        exited(&response)
    }

    /// Image blobs是否存在
    pub fn blobs_exited(&mut self, name: &str, digest: &str) -> Result<bool> {
        let path = format!("/v2/{}/blobs/{}", name, digest);
        let scope = Some(name.to_string());
        let response = self.reg_client.borrow_mut().head_request_registry(&path, &scope)?;
        exited(&response)
    }

    pub fn blobs_download(&self, name: &str, digest: &str) -> Result<Option<RegDownloader>> {
        let path = format!("/v2/{}/blobs/{}", name, digest);
        let blobs_cache_path = self.home_dir.cache.blobs.path.clone();
        let file_name = digest.replace(":", "_");
        let file_path = blobs_cache_path.join(file_name.as_str());
        if !self.download_check(file_name.as_str(), file_path.as_path())? {
            return Ok(None);
        }
        let downloader = self.reg_client.borrow().download(&path, file_path.to_str().unwrap())?;
        Ok(Some(downloader))
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
