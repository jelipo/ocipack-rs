use serde::Deserialize;
use serde::Serialize;

use crate::reg::manifest::{CommonManifestConfig, CommonManifestLayer};
use crate::reg::{FindPlatform, Layer, LayerConvert, Platform};

pub mod image;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciManifest {
    pub schema_version: usize,
    pub media_type: Option<String>,
    pub config: CommonManifestConfig,
    pub layers: Vec<CommonManifestLayer>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciManifestIndex {
    pub schema_version: usize,
    pub media_type: Option<String>,
    pub manifests: Vec<OciManifestIndexItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciManifestIndexItem {
    pub media_type: Option<String>,
    pub size: usize,
    pub digest: String,
    pub platform: OciManifestPlatform,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OciManifestPlatform {
    pub architecture: String,
    pub os: String,
    #[serde(rename = "os.version")]
    pub os_version: String,
    #[serde(rename = "os.features")]
    pub os_features: Vec<String>,
    pub variant: Option<String>,
    pub features: Vec<String>,
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

impl FindPlatform for OciManifestIndex {
    fn find_platform_digest(&self, platform: &Platform) -> Option<String> {
        self.manifests.iter().find(|&item| {
            item.platform.os == platform.os && item.platform.architecture == platform.arch
        }).map(|item| item.digest)
    }
}