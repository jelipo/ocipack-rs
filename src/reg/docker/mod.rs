use serde::Deserialize;
use serde::Serialize;

use crate::reg::{Layer, LayerConvert};
use crate::reg::manifest::{CommonManifestConfig, CommonManifestLayer};

pub mod image;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifest {
    pub schema_version: usize,
    pub media_type: String,
    pub config: CommonManifestConfig,
    pub layers: Vec<CommonManifestLayer>,
}

impl LayerConvert for DockerManifest {
    fn get_layers(&self) -> Vec<Layer> {
        self.layers.iter().map(|docker| Layer {
            media_type: &docker.media_type,
            size: docker.size,
            digest: &docker.digest,
        }).collect::<Vec<Layer>>()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifestConfig {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}
