use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::container::image::{History, Rootfs};
use crate::container::manifest::{CommonManifestConfig, CommonManifestLayer};
use crate::container::{ConfigBlob, FindPlatform, Layer, LayerConvert, Platform};

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
        self.manifests.iter().find(|&item| {
            let variant_match = match &platform.variant {
                None => item.platform.variant.is_none(),
                Some(variant) => match &item.platform.variant {
                    None => false,
                    Some(item_variant) => *item_variant == *variant,
                },
            };
            item.platform.os == platform.os && item.platform.architecture == platform.arch && variant_match
        }).map(|item| item.digest.clone())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciConfigBlob {
    pub created: Option<String>,
    pub author: Option<String>,
    pub architecture: Option<String>,
    pub os: Option<String>,
    pub config: Config,
    pub rootfs: Rootfs,
    pub history: Vec<History>,
}

impl ConfigBlob for OciConfigBlob {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(rename = "User")]
    pub user: Option<String>,
    #[serde(rename = "ExposedPorts")]
    pub exposed_ports: Option<HashMap<String, Value>>,
    #[serde(rename = "Env")]
    pub env: Option<Vec<String>>,
    #[serde(rename = "Entrypoint")]
    pub entrypoint: Option<Vec<String>>,
    #[serde(rename = "Cmd")]
    pub cmd: Option<Vec<String>>,
    #[serde(rename = "Volumes")]
    pub volumes: Option<HashMap<String, Value>>,
    #[serde(rename = "WorkingDir")]
    pub working_dir: Option<String>,
    #[serde(rename = "Labels")]
    pub labels: Option<HashMap<String, String>>,
    #[serde(rename = "Memory")]
    pub memory: Option<u64>,
    #[serde(rename = "MemorySwap")]
    pub memory_swap: Option<u64>,
    #[serde(rename = "CpuShares")]
    pub cpu_shares: Option<u64>,
    #[serde(rename = "Healthcheck")]
    pub healthcheck: Option<Value>,
}
