use anyhow::{Error, Result};
use serde::Deserialize;
use serde::Serialize;

use crate::reg::{Layer, LayerConvert, Reference, RegDigest};
use crate::reg::http::client::{RawRegistryResponse, RegistryHttpClient, RegistryResponse};

pub mod image;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciManifest {
    pub schema_version: usize,
    pub media_type: Option<String>,
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
