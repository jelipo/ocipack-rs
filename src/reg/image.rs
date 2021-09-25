use std::borrow::Borrow;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use anyhow::Result;
use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;

use crate::reg::client::HttpClient;
use crate::reg::Reference;

pub struct ImageManager {
    registry_addr: String,
    reg_client: Rc<HttpClient>,
}

impl ImageManager {
    pub fn new(registry_addr: String, client: Rc<HttpClient>) -> ImageManager {
        ImageManager {
            registry_addr,
            reg_client: client,
        }
    }

    pub fn get_manifests(&self, reference: &Reference) -> Result<Manifest2> {
        let path = format!("/v2/{}/manifests/{}", reference.image_name, reference.reference);

        self.reg_client.request::<u8, Manifest2>(&path, Method::GET, None)
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