use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::reg::{Layer, LayerConvert};
use crate::reg::manifest::{CommonManifestConfig, CommonManifestLayer};

pub mod image;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciImageIndex {
    pub schema_version: usize,
    pub media_type: Option<String>,
    pub manifests: Vec<OciImageIndexManifest>,
    pub annotations: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciImageIndexManifest {
    pub media_type: Option<String>,
    pub size: usize,
    pub digest: String,
    pub platform: Option<Platform>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub architecture: String,
    pub os: String,
    #[serde(rename = "os.version")]
    pub os_version: Option<String>,
    #[serde(rename = "os.features")]
    pub os_features: Option<Vec<String>>,
    pub variant: Option<String>,
    pub features: Option<String>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciManifest {
    pub schema_version: usize,
    pub media_type: Option<String>,
    pub config: CommonManifestConfig,
    pub layers: Vec<CommonManifestLayer>,
}

impl LayerConvert for OciManifest {
    fn get_layers(&self) -> Vec<Layer> {
        self.layers
            .iter()
            .map(|oci| Layer {
                media_type: &oci.media_type,
                size: oci.size,
                digest: &oci.digest,
            })
            .collect::<Vec<Layer>>()
    }
}
