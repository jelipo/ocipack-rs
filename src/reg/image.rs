use std::rc::Rc;

use anyhow::Result;
use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;

use crate::reg::client::{RegistryHttpClient, RegistryResponse};
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

    pub fn get_manifests(&self, refe: &Reference) -> Result<Manifest2> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        self.reg_client.request_registry::<u8, Manifest2>(&path, Method::GET, None)
    }

    pub fn exited(&self, refe: &Reference) -> Result<bool> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let response = self.reg_client.head_request_registry(&path);
        response.success()?
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
