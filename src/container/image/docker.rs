use std::collections::HashMap;

use crate::container::image::{History, Rootfs};
use crate::container::manifest::{CommonManifestConfig, CommonManifestLayer};
use crate::container::{ConfigBlob, FindPlatform, Layer, LayerConvert, Platform};
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

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
        self.layers
            .iter()
            .map(|docker| Layer {
                media_type: &docker.media_type,
                size: docker.size,
                digest: &docker.digest,
            })
            .collect::<Vec<Layer>>()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifestList {
    pub schema_version: usize,
    pub media_type: Option<String>,
    pub manifests: Vec<DockerManifestIndexItem>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifestIndexItem {
    pub media_type: Option<String>,
    pub size: usize,
    pub digest: String,
    pub platform: DockerManifestPlatform,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifestPlatform {
    pub architecture: String,
    pub os: String,
    #[serde(rename = "os.version")]
    pub os_version: Option<String>,
    #[serde(rename = "os.features")]
    pub os_features: Option<Vec<String>>,
    pub variant: Option<String>,
    pub features: Option<Vec<String>>,
}

impl FindPlatform for DockerManifestList {
    fn find_platform_digest(&self, platform: &Platform) -> Option<String> {
        let mut filter =
            self.manifests.iter().filter(|&item| item.platform.os == platform.os && item.platform.architecture == platform.arch);
        let possible_variants = platform.possible_variant();
        filter.find(|&item| possible_variants.contains(&item.platform.variant.clone().unwrap_or_default())).map(|item| item.digest.clone())
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockerConfigBlob {
    pub created: Option<String>,
    pub author: Option<String>,
    pub architecture: Option<String>,
    pub os: Option<String>,
    pub config: Config,
    pub rootfs: Rootfs,
    pub history: Vec<History>,
}

impl ConfigBlob for DockerConfigBlob {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(rename = "User")]
    pub user: Option<String>,
    #[serde(rename = "Memory")]
    pub memory: Option<u64>,
    #[serde(rename = "MemorySwap")]
    pub memory_swap: Option<u64>,
    #[serde(rename = "CpuShares")]
    pub cpu_shares: Option<u64>,
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
}
