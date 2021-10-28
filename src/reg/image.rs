use std::rc::Rc;
use std::thread::sleep;
use std::time::Duration;

use anyhow::{Error, Result};
use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;

use crate::reg::client::{RegistryHttpClient, RegistryResponse, SimpleRegistryResponse};
use crate::reg::Reference;

pub struct ImageManager {
    registry_addr: String,
    reg_client: Rc<RegistryHttpClient>,
}

impl ImageManager {
    pub fn new(registry_addr: String, client: Rc<RegistryHttpClient>) -> ImageManager {
        ImageManager {
            registry_addr,
            reg_client: client,
        }
    }

    /// 获取Image的Manifest
    pub fn manifests(&self, refe: &Reference) -> Result<Manifest2> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        self.reg_client.request_registry::<u8, Manifest2>(&path, Method::GET, None)
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

    pub fn blobs_download(&self, name: &str, digest: &str) -> Result<()> {
        let path = format!("/v2/{}/blobs/{}", name, digest);
        let mut downloader = self.reg_client.download(&path)?;
        let handle = downloader.start()?;
        let download_temp_mutex = downloader.download_temp();
        loop {
            sleep(Duration::from_secs(1));
            let download_temp = download_temp_mutex.lock().unwrap();
            println!("下载了 {}MiB", download_temp.curr_size / 1024 / 1024);
            if download_temp.done {
                println!("下载完成 {}字节", download_temp.curr_size);
                break;
            }
        }
        Ok(())
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
