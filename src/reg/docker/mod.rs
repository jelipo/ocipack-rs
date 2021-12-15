use std::cell::RefCell;
use std::fs::File;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::{Error, Result};
use log::info;
use reqwest::Method;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

use crate::reg::{BlobConfig, Layer, LayerConvert, Reference, RegDigest};
use crate::reg::docker::image::DockerConfigBlob;
use crate::reg::home::HomeDir;
use crate::reg::http::auth::TokenType;
use crate::reg::http::client::{ClientRequest, RawRegistryResponse, RegistryHttpClient, RegistryResponse};
use crate::reg::http::download::RegDownloader;
use crate::reg::http::upload::RegUploader;
use crate::reg::RegContentType;

pub mod image;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifest {
    pub schema_version: usize,
    pub media_type: String,
    pub config: DockerManifestConfig,
    pub layers: Vec<DockerManifestLayer>,
}

impl LayerConvert for DockerManifest {
    fn to_layers(&self) -> Vec<Layer> {
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DockerManifestLayer {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}


