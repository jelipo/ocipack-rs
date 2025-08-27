use anyhow::{anyhow, Result};
use async_compression::tokio::write::{GzipDecoder, GzipEncoder, ZstdDecoder};
use async_compression::Level::Fastest;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use log::info;
use serde::{Deserialize, Serialize};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};
use tokio::fs::{create_dir_all, File};
use tokio::io;
use tokio::io::{AsyncBufRead, AsyncRead, AsyncWrite};
use tokio_tar::{Builder, Header};

use crate::container::manifest::{CommonManifestConfig, Manifest};
use crate::container::{CompressType, ConfigBlobSerialize, RegContentType, RegDigest};
use crate::util::sha::bytes_sha256;
use crate::GLOBAL_CONFIG;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageIndex {
    pub schema_version: usize,
    pub manifests: Vec<CommonManifestConfig>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TarManifestJson {
    pub config: String,
    pub repo_tags: Vec<String>,
    pub layers: Vec<String>,
}

pub struct TarTargetAdapter {
    pub image_raw_name: String,
    pub target_manifest: Manifest,
    pub manifest_raw: String,
    pub target_config_blob_serialize: ConfigBlobSerialize,
    pub save_path: PathBuf,
    pub use_gzip: bool,
}

impl TarTargetAdapter {
    pub async fn save(self) -> Result<()> {
        info!("start saving image as file");
        if self.save_path.exists() {
            return Err(anyhow!("file already exists: {:?}", self.save_path));
        }
        if let Some(parent) = self.save_path.parent() {
            create_dir_all(parent).await?;
        }
        let output_file = File::create(self.save_path).await?;
        let write_box: Box<dyn AsyncWrite + Send + Unpin> = match self.use_gzip {
            true => Box::new(GzipEncoder::with_quality(output_file, Fastest)),
            false => Box::new(output_file),
        };
        let mut builder = Builder::new(write_box);

        let (manifest_media_type, layers) = match self.target_manifest {
            Manifest::OciV1(oci) => (RegContentType::OCI_MANIFEST.0.to_string(), oci.layers.clone()),
            Manifest::DockerV2S2(dockerv2s2) => (RegContentType::DOCKER_MANIFEST.0.to_string(), dockerv2s2.layers.clone()),
        };
        // 解析所有layer为tar
        let mut layer_sha_vec = Vec::<String>::with_capacity(layers.len());
        for comm_layer in layers.iter() {
            let digest = RegDigest::new_with_digest(comm_layer.digest.clone());
            let layer = GLOBAL_CONFIG
                .home_dir
                .cache
                .blobs
                .local_layer(&digest)
                .ok_or_else(|| anyhow!("can not found this layer: {}", digest.sha256))?;
            let layer_file = File::open(layer.layer_file_path).await?;
            let (size, layer_reader) = match layer.compress_type {
                CompressType::Tar => (layer_file.metadata().await?.len(), Box::new(layer_file)),
                CompressType::Tgz => {
                    let mut temp_file = tempfile::Builder::new().tempfile_in(&GLOBAL_CONFIG.home_dir.cache.temp_dir)?;
                    let mut temp_file = unsafe { File::from_raw_fd(temp_file.as_raw_fd()) };
                    let mut decoder = GzipDecoder::new(layer_file);
                    let size = io::copy(&mut decoder, &mut temp_file).await?;
                    (size, Box::new(temp_file))
                }
                CompressType::Zstd => {
                    let mut temp_file = tempfile::Builder::new().tempfile_in(&GLOBAL_CONFIG.home_dir.cache.temp_dir)?;
                    let mut temp_file = unsafe { File::from_raw_fd(temp_file.as_raw_fd()) };
                    let mut decoder = ZstdDecoder::new(layer_file);
                    let size = io::copy(&mut decoder, &mut temp_file).await?;
                    (size, Box::new(temp_file))
                }
            };
            let mut header = Header::new_gnu();
            let layer_path = format!("blobs/sha256/{}", layer.diff_layer_sha);
            header.set_path(&layer_path)?;
            header.set_size(size);
            header.set_mode(0o644);
            header.set_cksum();
            builder.append(&header, layer_reader).await?;
            layer_sha_vec.push(layer_path);
        }

        // 将config blob 也加入到layer中
        let config_blob = self.target_config_blob_serialize;
        //layer_sha_vec.push(config_blob.digest.sha256.clone());
        let config_blob_path = format!("blobs/sha256/{}", config_blob.digest.sha256);
        write_string_to_builder(config_blob.json_str, config_blob_path.clone(), &mut builder).await?;
        // 写入 index.json

        let manifest_digest = RegDigest::new_with_sha256(bytes_sha256(self.manifest_raw.as_bytes()));
        let image_index = ImageIndex {
            schema_version: 2,
            manifests: vec![CommonManifestConfig {
                media_type: manifest_media_type.clone(),
                size: manifest_media_type.as_bytes().len() as u64,
                digest: manifest_digest.digest,
            }],
        };
        write_string_to_builder(serde_json::to_string(&image_index)?, "index.json", &mut builder).await?;
        // 写入 manifest.json
        let manifest_json = TarManifestJson {
            config: config_blob_path,
            repo_tags: vec![self.image_raw_name.clone()],
            layers: layer_sha_vec,
        };
        write_string_to_builder(serde_json::to_string(&vec![manifest_json])?, "manifest.json", &mut builder).await?;
        // 写入 oci-layout
        write_string_to_builder(r#"{"imageLayoutVersion":"1.0.0"}"#.to_string(), "oci-layout", &mut builder).await?;
        // 写入manifest
        let manifest_path = format!("blobs/sha256/{}", manifest_digest.sha256);
        write_string_to_builder(self.manifest_raw, manifest_path, &mut builder).await?;
        builder.finish().await?;
        Ok(())
    }
}

async fn write_string_to_builder<P: AsRef<Path>>(data: String, path: P, builder: &mut Builder<Box<dyn AsyncWrite + Send + Unpin>>) -> Result<()> {
    let size = data.as_bytes().len() as u64;
    let mut header = Header::new_gnu();
    header.set_path(path)?;
    header.set_size(size);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append(&header, data.as_bytes()).await?;
    Ok(())
}
