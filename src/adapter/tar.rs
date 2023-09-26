use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use bytes::Buf;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use tar::{Builder, Header};

use crate::adapter::{ImageInfo, TargetImageAdapter, TargetInfo};
use crate::config::cmd::TargetFormat;
use crate::container::{CompressType, ConfigBlobSerialize, RegDigest};
use crate::container::manifest::{CommonManifestConfig, Manifest};
use crate::container::oci::OciManifest;
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
    info: TargetInfo,
    target_manifest: Manifest,
    target_config_blob_serialize: ConfigBlobSerialize,
    save_path: PathBuf,
    use_gzip: bool,
}

impl TargetImageAdapter for TarTargetAdapter {
    fn info(&self) -> &TargetInfo {
        &self.info
    }
}


impl TarTargetAdapter {
    pub fn save(self) -> Result<()> {
        let output_file = File::create(self.save_path)?;
        let write_box: Box<dyn Write> = match self.use_gzip {
            true => Box::new(GzEncoder::new(output_file, Compression::fast())),
            false => Box::new(output_file),
        };
        let mut builder = Builder::new(write_box);

        let (manifest_config, layers) = match self.target_manifest {
            Manifest::OciV1(oci) => (oci.config.clone(), oci.layers.clone()),
            Manifest::DockerV2S2(dockerv2s2) => (dockerv2s2.config.clone(), dockerv2s2.layers.clone()),
        };
        let image_index = ImageIndex {
            schema_version: 2,
            manifests: vec![manifest_config],
        };
        // TODO

        // 解析所有layer为tar
        let mut layer_reader_vec = layers.iter().map(|comm_layer| {
            let digest = RegDigest::new_with_digest(comm_layer.digest.clone());
            let layer = GLOBAL_CONFIG.home_dir.cache.blobs.local_layer(&digest)
                .ok_or_else(|| anyhow!("can not found this layer: {}",digest.sha256))?;
            let layer_file = File::open(layer.layer_file_path)?;
            let layer_reader: Box<dyn Read> = match layer.compress_type {
                CompressType::Tar => Box::new(layer_file),
                CompressType::Tgz => Box::new(GzDecoder::new(layer_file)),
                CompressType::Zstd => Box::new(zstd::stream::Decoder::new(layer_file)?),
            };
            Ok((layer.diff_layer_sha, layer_reader))
        }).collect::<Result<Vec<_>>>()?;
        // 将config blob 也加入
        let config_blob = self.target_config_blob_serialize;
        layer_reader_vec.push((config_blob.digest.digest, Box::new(Cursor::new(config_blob.json_str))));

        let image_raw_name = self.info.image_info.image_raw_name
            .ok_or_else(|| anyhow!("must set a image raw name"))?;
        let layer_sha_vec = layer_reader_vec.into_iter().map(|(layer_sha, layer_reader)| {
            let mut header = Header::new_gnu();
            header.set_path(format!("blobs/sha256/{}", layer_sha))?;
            builder.append(&header, layer_reader)?;
            Ok(layer_sha)
        }).collect::<Result<Vec<_>>>()?;
        TarManifestJson {
            config: format!("blobs/sha256/{}", config_blob.digest.sha256),
            repo_tags: vec![image_raw_name],
            layers: layer_sha_vec,
        };
        // TODO
        Ok(())
    }
}

#[test]
fn it_works() -> Result<()> {
    let adapter = TarTargetAdapter {
        info: TargetInfo {
            image_info: ImageInfo {
                image_raw_name: None,
                image_host: "".to_string(),
                image_name: "".to_string(),
                reference: "".to_string(),
            },
            format: TargetFormat::Docker,
        },
        target_manifest: Manifest::OciV1(OciManifest {
            schema_version: 0,
            media_type: None,
            config: CommonManifestConfig {
                media_type: "".to_string(),
                size: 0,
                digest: "".to_string(),
            },
            layers: vec![],
        }),

        target_config_blob_serialize: ConfigBlobSerialize {
            json_str: "".to_string(),
            digest: RegDigest { sha256: "".to_string(), digest: "".to_string() },
            size: 0,
        },
        save_path: Default::default(),
        use_gzip: false,
    };
    adapter.save()
}