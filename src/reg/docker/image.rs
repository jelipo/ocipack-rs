use serde::Deserialize;
use serde::Serialize;


#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigBlob {
    pub created: Option<String>,
    pub author: Option<String>,
    pub architecture: String,
    pub os: Option<String>,
    pub config: Config,
    pub rootfs: Rootfs,
    pub history: Vec<History>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(rename = "User")]
    pub user: String,
    #[serde(rename = "Memory")]
    pub memory: Option<u64>,
    #[serde(rename = "MemorySwap")]
    pub memory_swap: Option<u64>,
    #[serde(rename = "CpuShares")]
    pub cpu_shares: Option<u64>,
    #[serde(rename = "ExposedPorts")]
    pub exposed_ports: Option<ExposedPorts>,
    #[serde(rename = "Env")]
    pub env: Vec<String>,
    #[serde(rename = "Entrypoint")]
    pub entrypoint: Vec<String>,
    #[serde(rename = "Cmd")]
    pub cmd: Vec<String>,
    #[serde(rename = "Volumes")]
    pub volumes: Option<Volumes>,
    #[serde(rename = "WorkingDir")]
    pub working_dir: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExposedPorts {
    #[serde(rename = "8080/tcp")]
    pub n8080_tcp: Option<N8080tcp>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct N8080tcp {}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Volumes {
    #[serde(rename = "/var/job-result-data")]
    pub var_job_result_data: Option<VarJobResultData>,
    #[serde(rename = "/var/log/my-app-logs")]
    pub var_log_my_app_logs: Option<VarLogMyAppLogs>,
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
