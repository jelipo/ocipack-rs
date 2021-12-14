use std::collections::HashMap;
use std::hash::Hash;

use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;

use crate::reg::ConfigBlob;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OciConfigBlob {
    pub created: Option<String>,
    pub author: Option<String>,
    pub architecture: String,
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
    pub labels: Option<Vec<HashMap<String, String>>>,
    #[serde(rename = "Memory")]
    pub memory: Option<u64>,
    #[serde(rename = "MemorySwap")]
    pub memory_swap: Option<u64>,
    #[serde(rename = "CpuShares")]
    pub cpu_shares: Option<u64>,
    #[serde(rename = "Healthcheck")]
    pub healthcheck: Option<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VarJobResultData {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VarLogMyAppLogs {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rootfs {
    #[serde(rename = "diff_ids")]
    pub diff_ids: Vec<String>,
    #[serde(rename = "type")]
    pub type_field: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    pub created: String,
    #[serde(rename = "created_by")]
    pub created_by: String,
    #[serde(rename = "empty_layer")]
    pub empty_layer: Option<bool>,
}
