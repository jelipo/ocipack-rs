use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Error, Result};
use log::info;
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};
use url::Url;

use manifest::Manifest;

use crate::GLOBAL_CONFIG;
use crate::reg::docker::DockerManifest;
use crate::reg::docker::image::DockerConfigBlob;
use crate::reg::http::auth::TokenType;
use crate::reg::http::client::{ClientRequest, RawRegistryResponse, RegistryHttpClient, RegistryResponse};
use crate::reg::http::download::RegDownloader;
use crate::reg::http::RegistryAuth;
use crate::reg::http::upload::RegUploader;
use crate::reg::oci::image::OciConfigBlob;
use crate::reg::oci::OciManifest;
use crate::reg::proxy::ProxyInfo;
use crate::util::sha::bytes_sha256;

pub mod docker;
pub mod home;
pub mod http;
pub mod manifest;
pub mod oci;
pub mod proxy;

pub struct Reference<'a> {
    /// Image的名称
    pub image_name: &'a str,
    /// 可以是TAG或者digest
    pub reference: &'a str,
}

pub struct BlobConfig {
    pub file_path: Box<Path>,
    pub file_name: String,
    pub reg_digest: RegDigest,
    pub short_hash: String,
}

impl BlobConfig {
    pub fn new(file_path: Box<Path>, file_name: String, digest: RegDigest) -> BlobConfig {
        BlobConfig {
            file_path,
            file_name,
            short_hash: digest.sha256[..12].to_string(),
            reg_digest: digest,
        }
    }
}

#[derive(Clone)]
pub struct RegDigest {
    pub sha256: String,
    pub digest: String,
}

impl RegDigest {
    pub fn new_with_sha256(sha256: String) -> RegDigest {
        RegDigest {
            digest: format!("sha256:{}", &sha256),
            sha256,
        }
    }

    pub fn new_with_digest(digest: String) -> RegDigest {
        RegDigest {
            sha256: digest.as_str()[7..].to_string(),
            digest,
        }
    }
}

pub struct Registry {
    pub image_manager: MyImageManager,
}

pub struct RegistryCreateInfo {
    pub auth: Option<RegistryAuth>,
    pub conn_timeout_second: u64,
    pub proxy: Option<ProxyInfo>,
}

impl Registry {
    pub fn open(use_https: bool, host: &str, reg_cteate_info: RegistryCreateInfo) -> Result<Registry> {
        let reg_addr = format!("{}{}", if use_https { "https://" } else { "http://" }, host);
        let client = RegistryHttpClient::new(reg_addr, reg_cteate_info.auth, reg_cteate_info.conn_timeout_second, reg_cteate_info.proxy)?;
        let image = MyImageManager::new(client);
        Ok(Registry { image_manager: image })
    }
}

pub struct MyImageManager {
    reg_client: RegistryHttpClient,
}

impl MyImageManager {
    pub fn new(client: RegistryHttpClient) -> MyImageManager {
        MyImageManager { reg_client: client }
    }

    /// 获取Image的Manifest
    pub fn manifests(&mut self, refe: &Reference) -> Result<Manifest> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name);
        let accepts = &[RegContentType::OCI_MANIFEST, RegContentType::DOCKER_MANIFEST];
        let request: ClientRequest<u8> = ClientRequest::new_get_request(&path, scope, accepts);
        let response = self.reg_client.simple_request(request)?;
        let content_type = response.content_type().ok_or_else(|| Error::msg("manifest content-type header not found"))?;
        if RegContentType::DOCKER_MANIFEST.val() == content_type {
            let manifest = serde_json::from_str::<DockerManifest>(&response.string_body())?;
            Ok(Manifest::DockerV2S2(manifest))
        } else if RegContentType::OCI_MANIFEST.val() == content_type {
            let manifest = serde_json::from_str::<OciManifest>(&response.string_body())?;
            Ok(Manifest::OciV1(manifest))
        } else {
            let msg = format!(
                "unknown content-type:{},body:{}",
                content_type.to_string(),
                response.string_body()
            );
            Err(Error::msg(msg))
        }
    }

    /// Image blobs是否存在
    pub fn blobs_exited(&mut self, name: &str, blob_digest: &RegDigest) -> Result<bool> {
        let path = format!("/v2/{}/blobs/{}", name, blob_digest.digest);
        let scope = Some(name);
        let request = ClientRequest::new_head_request(&path, scope, TokenType::Pull);
        let response = self.reg_client.simple_request::<u8>(request)?;
        exited(&response)
    }

    pub fn config_blob<T: ConfigBlob + DeserializeOwned>(&mut self, name: &str, blob_digest: &str) -> Result<T> {
        let url_path = format!("/v2/{}/blobs/{}", name, blob_digest);
        let accepts = &[RegContentType::OCI_IMAGE_CONFIG, RegContentType::DOCKER_CONTAINER_IMAGE];
        let request: ClientRequest<u8> = ClientRequest::new_get_request(&url_path, Some(name), accepts);
        let response = self.reg_client.simple_request(request)?;
        let str_body = response.string_body();
        Ok(serde_json::from_str::<T>(&str_body)?)
    }

    pub fn layer_blob_download(&mut self, name: &str, blob_digest: &RegDigest, layer_size: Option<u64>) -> Result<RegDownloader> {
        let url_path = format!("/v2/{}/blobs/{}", name, blob_digest.digest);
        let file_path = GLOBAL_CONFIG.home_dir.cache.blobs.download_ready(blob_digest);
        let file_name = blob_digest.sha256.clone();
        let mut blob_config = BlobConfig::new(file_path, file_name, blob_digest.clone());
        if let Some(local) = GLOBAL_CONFIG.home_dir.cache.blobs.local_layer(blob_digest) {
            let layer_len = local.layer_file_path.metadata()?.len();
            blob_config.file_path = local.layer_file_path.into_boxed_path();
            let finished = RegDownloader::new_finished(blob_config, layer_len)?;
            return Ok(finished);
        }
        let downloader = self.reg_client.download(&url_path, blob_config, name, layer_size)?;
        Ok(downloader)
    }

    /// 上传layer类型的blob文件
    pub fn layer_blob_upload(&mut self, name: &str, blob_digest: &RegDigest, file_local_path: &str) -> Result<RegUploader> {
        let file_path = PathBuf::from(file_local_path).into_boxed_path();
        let file_name = file_path.file_name().expect("file name error").to_str().unwrap().to_string();
        let blob_config = BlobConfig::new(file_path.clone(), file_name, blob_digest.clone());
        if self.blobs_exited(name, blob_digest)? {
            return Ok(RegUploader::new_finished_uploader(
                blob_config,
                file_path.metadata()?.len(),
                "blob exists in registry".to_string(),
            ));
        }
        let mut location_url = self.layer_blob_upload_ready(name)?;
        location_url.query_pairs_mut().append_pair("digest", &blob_digest.digest);
        let blob_upload_url = location_url.as_str();
        info!("blob_upload_url is {}", blob_upload_url);
        let reg_uploader = self.reg_client.upload(location_url.to_string(), blob_config, name, &file_path)?;
        Ok(reg_uploader)
    }

    /// 向仓库获取上传blob的URL
    pub fn layer_blob_upload_ready(&mut self, name: &str) -> Result<Url> {
        let url_path = format!("/v2/{}/blobs/uploads/", name);
        let scope = Some(name);
        let request = ClientRequest::new(&url_path, scope, Method::POST, &[], None, TokenType::PushAndPull);
        let success_resp = self.reg_client.request_full_response::<u8>(request)?;
        let location = success_resp.location_header().expect("location header not found");
        let url = Url::parse(location)?;
        Ok(url)
    }

    pub fn put_manifest(&mut self, refe: &Reference, manifest: Manifest) -> Result<String> {
        let path = format!("/v2/{}/manifests/{}", refe.image_name, refe.reference);
        let scope = Some(refe.image_name);
        let response = match manifest {
            Manifest::OciV1(oci_manifest) => {
                let request = ClientRequest::new_with_content_type(
                    &path,
                    scope,
                    Method::PUT,
                    &[],
                    Some(&oci_manifest),
                    &RegContentType::OCI_MANIFEST,
                    TokenType::PushAndPull,
                );
                self.reg_client.simple_request::<OciManifest>(request)?
            }
            Manifest::DockerV2S2(docker_v2s2_manifest) => {
                let request = ClientRequest::new_with_content_type(
                    &path,
                    scope,
                    Method::PUT,
                    &[],
                    Some(&docker_v2s2_manifest),
                    &RegContentType::DOCKER_MANIFEST,
                    TokenType::PushAndPull,
                );
                self.reg_client.simple_request::<DockerManifest>(request)?
            }
        };
        Ok(response.string_body())
    }
}

fn exited(simple_response: &RawRegistryResponse) -> Result<bool> {
    match simple_response.status_code() {
        200..300 => Ok(true),
        404 => Ok(false),
        status_code => {
            let msg = format!("request registry error,status code:{}", status_code);
            Err(Error::msg(msg))
        }
    }
}

pub trait LayerConvert {
    fn get_layers(&self) -> Vec<Layer>;
}

pub struct Layer<'a> {
    pub media_type: &'a str,
    pub size: u64,
    pub digest: &'a str,
}

pub trait ConfigBlob {}

#[derive(Clone)]
pub enum ConfigBlobEnum {
    OciV1(OciConfigBlob),
    DockerV2S2(DockerConfigBlob),
}

impl ConfigBlobEnum {
    pub fn add_diff_layer(&mut self, new_tar_digest: String) {
        match self {
            ConfigBlobEnum::OciV1(oci) => oci.rootfs.diff_ids.insert(0, new_tar_digest),
            ConfigBlobEnum::DockerV2S2(docker) => docker.rootfs.diff_ids.insert(0, new_tar_digest),
        }
    }

    pub fn add_labels(&mut self, new_labels: HashMap<String, String>) {
        if new_labels.is_empty() {
            return;
        }
        match self {
            ConfigBlobEnum::OciV1(oci) => match &mut oci.config.labels {
                None => oci.config.labels = Some(new_labels),
                Some(source) => source.extend(new_labels),
            },
            ConfigBlobEnum::DockerV2S2(_) => (),
        };
    }

    pub fn add_envs(&mut self, envs: HashMap<String, String>) {
        if envs.is_empty() {
            return;
        }
        let new_envs = envs.into_iter().map(|(k, v)| format!("{}={}", k, v)).collect::<Vec<String>>();
        match self {
            ConfigBlobEnum::OciV1(oci) => match &mut oci.config.env {
                None => oci.config.env = Some(new_envs),
                Some(source) => source.extend(new_envs),
            },
            ConfigBlobEnum::DockerV2S2(docker) => match &mut docker.config.env {
                None => docker.config.env = Some(new_envs),
                Some(source) => source.extend(new_envs),
            },
        };
    }

    pub fn overwrite_cmd(&mut self, cmds: Vec<String>) {
        match self {
            ConfigBlobEnum::OciV1(oci) => oci.config.cmd = Some(cmds),
            ConfigBlobEnum::DockerV2S2(docker) => docker.config.cmd = Some(cmds),
        }
    }

    pub fn add_ports(&mut self, port_exposes: Vec<String>) {
        if port_exposes.is_empty() {
            return;
        }
        let mut map = HashMap::<String, Value>::with_capacity(port_exposes.len());
        port_exposes.into_iter().for_each(|expose| {
            map.insert(expose, Value::Object(Map::new()));
        });
        match self {
            ConfigBlobEnum::OciV1(oci) => match &mut oci.config.exposed_ports {
                None => oci.config.exposed_ports = Some(map),
                Some(source) => source.extend(map),
            },
            ConfigBlobEnum::DockerV2S2(docker) => match &mut docker.config.exposed_ports {
                None => docker.config.exposed_ports = Some(map),
                Some(source) => source.extend(map),
            },
        }
    }

    pub fn overwrite_work_dir(&mut self, work_dir: String) {
        match self {
            ConfigBlobEnum::OciV1(oci) => oci.config.working_dir = Some(work_dir),
            ConfigBlobEnum::DockerV2S2(docker) => docker.config.working_dir = Some(work_dir),
        }
    }

    pub fn overwrite_user(&mut self, user: String) {
        match self {
            ConfigBlobEnum::OciV1(oci) => oci.config.user = Some(user),
            ConfigBlobEnum::DockerV2S2(docker) => docker.config.user = Some(user),
        }
    }

    pub fn to_json_string(&self) -> Result<String> {
        Ok(match self {
            ConfigBlobEnum::OciV1(oci) => serde_json::to_string(oci),
            ConfigBlobEnum::DockerV2S2(docker) => serde_json::to_string(docker),
        }?)
    }

    pub fn serialize(&self) -> Result<ConfigBlobSerialize> {
        let json = self.to_json_string()?;
        let json_bytes = json.as_bytes();
        let digest = RegDigest::new_with_sha256(bytes_sha256(json_bytes));
        let size = json_bytes.len();
        Ok(ConfigBlobSerialize {
            json_str: json,
            digest,
            size: size as u64,
        })
    }
}

pub struct ConfigBlobSerialize {
    pub json_str: String,
    pub digest: RegDigest,
    pub size: u64,
}

pub struct RegContentType(pub &'static str);

impl RegContentType {
    /// Docker content-type
    pub const DOCKER_MANIFEST: Self = Self("application/vnd.docker.distribution.manifest.v2+json");
    pub const DOCKER_MANIFEST_LIST: Self = Self("application/vnd.docker.distribution.manifest.list.v2+json");
    pub const DOCKER_FOREIGN_LAYER_TGZ: Self = Self("application/vnd.docker.image.rootfs.foreign.diff.tar.gzip");
    pub const DOCKER_LAYER_TGZ: Self = Self("application/vnd.docker.image.rootfs.diff.tar.gzip");
    pub const DOCKER_CONTAINER_IMAGE: Self = Self("application/vnd.docker.container.image.v1+json");

    /// OCI content-type
    pub const OCI_MANIFEST: Self = Self("application/vnd.oci.image.manifest.v1+json");
    pub const OCI_LAYER_TAR: Self = Self("application/vnd.oci.image.layer.v1.tar");
    pub const OCI_LAYER_TGZ: Self = Self("application/vnd.oci.image.layer.v1.tar+gzip");
    pub const OCI_LAYER_ZSTD: Self = Self("application/vnd.oci.image.layer.v1.tar+zstd");
    pub const OCI_LAYER_NONDISTRIBUTABLE_TAR: Self = Self("application/vnd.oci.image.layer.nondistributable.v1.tar");
    pub const OCI_LAYER_NONDISTRIBUTABLE_TGZ: Self = Self("application/vnd.oci.image.layer.nondistributable.v1.tar+gzip");
    pub const OCI_LAYER_NONDISTRIBUTABLE_ZSTD: Self = Self("application/vnd.oci.image.layer.nondistributable.v1.tar+zstd");
    pub const OCI_IMAGE_CONFIG: Self = Self("application/vnd.oci.image.config.v1+json");

    pub const ALL: Self = Self("*/*");

    pub fn val(&self) -> &'static str {
        self.0
    }

    pub fn compress_type(media_type: &str) -> Result<CompressType> {
        if [
            RegContentType::OCI_LAYER_TAR.0,
            RegContentType::OCI_LAYER_NONDISTRIBUTABLE_TAR.0,
        ]
            .contains(&media_type)
        {
            Ok(CompressType::Tar)
        } else if [
            RegContentType::DOCKER_FOREIGN_LAYER_TGZ.0,
            RegContentType::OCI_LAYER_TGZ.0,
            RegContentType::DOCKER_LAYER_TGZ.0,
            RegContentType::OCI_LAYER_NONDISTRIBUTABLE_TGZ.0,
        ]
            .contains(&media_type)
        {
            Ok(CompressType::Tgz)
        } else if [
            RegContentType::OCI_LAYER_ZSTD.0,
            RegContentType::OCI_LAYER_NONDISTRIBUTABLE_ZSTD.0,
        ]
            .contains(&media_type)
        {
            Ok(CompressType::Zstd)
        } else {
            Err(Error::msg("not a layer media type"))
        }
    }
}

pub enum CompressType {
    Tar,
    Tgz,
    Zstd,
}

impl ToString for CompressType {
    fn to_string(&self) -> String {
        match self {
            CompressType::Tar => "TAR",
            CompressType::Tgz => "TGZ",
            CompressType::Zstd => "ZSTD",
        }
            .to_string()
    }
}

impl FromStr for CompressType {
    type Err = anyhow::Error;

    fn from_str(str: &str) -> std::result::Result<Self, Self::Err> {
        match str {
            "TAR" => Ok(CompressType::Tar),
            "TGZ" => Ok(CompressType::Tgz),
            "ZSTD" => Ok(CompressType::Zstd),
            _ => Err(Error::msg(format!("unknown compress type:{}", str))),
        }
    }
}
