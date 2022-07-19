use serde::Deserialize;
use serde::Serialize;

use crate::reg::manifest::{CommonManifestConfig, CommonManifestLayer};
use crate::reg::{Layer, LayerConvert, ManifestRaw};

pub mod image;

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