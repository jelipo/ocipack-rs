use std::rc::Rc;

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
    schema_version: usize,
    media_type: String,
    config: ManifestConfig,
    layers: Vec<ManifestLayer>,
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
    media_type: String,
    size: usize,
    digest: String,
    urls: Option<Vec<String>>,
}
