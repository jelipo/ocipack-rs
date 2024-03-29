use serde::Deserialize;
use serde::Serialize;

use crate::container::manifest::{CommonManifestConfig, CommonManifestLayer};
use crate::container::{FindPlatform, Layer, LayerConvert, Platform};

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
    pub os_version: Option<String>,
    #[serde(rename = "os.features")]
    pub os_features: Option<Vec<String>>,
    pub variant: Option<String>,
    pub features: Option<Vec<String>>,
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
        self.manifests
            .iter()
            .find(|&item| {
                let variant_match = match &platform.variant {
                    None => item.platform.variant.is_none(),
                    Some(variant) => match &item.platform.variant {
                        None => false,
                        Some(item_variant) => *item_variant == *variant,
                    },
                };
                item.platform.os == platform.os && item.platform.architecture == platform.arch && variant_match
            })
            .map(|item| item.digest.clone())
    }
}
