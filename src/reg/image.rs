use std::rc::Rc;

use anyhow::{Error, Result};
use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;

use crate::reg::home::HomeDir;
use crate::reg::http::client::{RegistryHttpClient, RegistryResponse, SimpleRegistryResponse};
use crate::reg::http::download::{CustomDownloadFileName, DownloadFilenameType, RegDownloader};
use crate::reg::Reference;
use crate::util::sha::{Sha, ShaType};

pub struct ImageManager {
    registry_addr: String,
    reg_client: Rc<RegistryHttpClient>,
    home_dir: Rc<HomeDir>,
}

impl ImageManager {
    pub fn new(
        registry_addr: String,
        client: Rc<RegistryHttpClient>,
        home_dir: Rc<HomeDir>,
    ) -> ImageManager {
        ImageManager {
            registry_addr,
            reg_client: client,
            home_dir,
        }
    }

    /// 获取Image的Manifest
    pub fn manifests(&self, refe: &Reference) -> Result<Manifest2> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        self.reg_client
            .request_registry::<u8, Manifest2>(&path, Method::GET, None)
    }

    /// Image manifests是否存在
    pub fn manifests_exited(&self, refe: &Reference) -> Result<bool> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let response = self.reg_client.head_request_registry(&path)?;
        exited(&response)
    }

    /// Image blobs是否存在
    pub fn blobs_exited(&self, name: &str, digest: &str) -> Result<bool> {
        let path = format!("/v2/{}/blobs/{}", name, digest);
        let response = self.reg_client.head_request_registry(&path)?;
        exited(&response)
    }

    pub fn blobs_download(&self, name: &str, digest: &str) -> Result<RegDownloader> {
        let path = format!("/v2/{}/blobs/{}", name, digest);
        let blobs_cache_path = self.home_dir.cache.blobs.path.clone();
        let filename_type = if digest.starts_with("sha256:") {
            let sha256 = digest.replace("sha256:", "");
            DownloadFilenameType::Custom(CustomDownloadFileName {
                dir_path: blobs_cache_path.join("sha256").into_boxed_path(),
                file_name: sha256.clone(),
                sha: Some(Sha {
                    sha_type: ShaType::Sha256,
                    sha_str: sha256.clone(),
                }),
            })
        } else {
            DownloadFilenameType::Custom(CustomDownloadFileName {
                dir_path: blobs_cache_path,
                file_name: digest.to_string(),
                sha: None,
            })
        };
        let downloader = self.reg_client.download(&path, filename_type)?;
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
