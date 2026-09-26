use anyhow::{Result, anyhow};
use async_compression::Level::Fastest;
use async_compression::tokio::bufread::{GzipDecoder, ZstdDecoder};
use async_compression::tokio::write::GzipEncoder;
use log::info;
use serde::{Deserialize, Serialize};
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use tokio::fs::{File, create_dir_all};
use tokio::io;
use tokio::io::{AsyncRead, AsyncSeekExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio_tar::{Builder, Header};

use crate::GLOBAL_CONFIG;
use crate::container::home::HomeDir;
use crate::container::manifest::{CommonManifestConfig, Manifest};
use crate::container::{CompressType, ConfigBlobSerialize, RegContentType, RegDigest};
use crate::util::sha::bytes_sha256;

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
    pub target_config_blob_serialize: ConfigBlobSerialize,
    pub save_path: PathBuf,
    pub use_gzip: bool,
}

impl TarTargetAdapter {
    pub async fn save(self) -> Result<()> {
        self.save_to(&GLOBAL_CONFIG.home_dir).await
    }

    async fn save_to(self, home_dir: &HomeDir) -> Result<()> {
        info!("start saving image as file");
        if self.save_path.exists() {
            return Err(anyhow!("file already exists: {:?}", self.save_path));
        }
        let parent = self.save_path.parent().filter(|path| !path.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        create_dir_all(parent).await?;
        let temp_path = tempfile::Builder::new().tempfile_in(parent)?.into_temp_path();
        let output_file = File::create(&temp_path).await?;
        let write_box: Box<dyn AsyncWrite + Send + Unpin> = match self.use_gzip {
            true => Box::new(GzipEncoder::with_quality(output_file, Fastest)),
            false => Box::new(output_file),
        };
        let mut builder = Builder::new(write_box);

        let (manifest_media_type, layers, manifest_raw) = match self.target_manifest {
            Manifest::OciV1(oci) => (
                RegContentType::OCI_MANIFEST.0.to_string(),
                oci.layers.clone(),
                serde_json::to_string(&oci)?,
            ),
            Manifest::DockerV2S2(docker) => (
                RegContentType::DOCKER_MANIFEST.0.to_string(),
                docker.layers.clone(),
                serde_json::to_string(&docker)?,
            ),
        };
        // 解析所有layer为tar
        let mut layer_sha_vec = Vec::<String>::with_capacity(layers.len());
        for comm_layer in layers.iter() {
            let digest = RegDigest::new_with_digest(comm_layer.digest.clone())?;
            let layer = home_dir.cache.blobs.local_layer(&digest).ok_or_else(|| anyhow!("can not found this layer: {}", digest.sha256))?;
            let compressed_path = format!("blobs/sha256/{}", digest.sha256);
            let mut compressed_header = Header::new_gnu();
            compressed_header.set_path(&compressed_path)?;
            compressed_header.set_size(tokio::fs::metadata(&layer.layer_file_path).await?.len());
            compressed_header.set_mode(0o644);
            compressed_header.set_cksum();
            builder.append(&compressed_header, File::open(&layer.layer_file_path).await?).await?;

            let layer_file = File::open(&layer.layer_file_path).await?;
            let (size, layer_reader) = uncompressed_layer(layer_file, layer.compress_type, &home_dir.cache.temp_dir).await?;
            let mut header = Header::new_gnu();
            let layer_path = format!("blobs/sha256/{}", layer.diff_layer_sha);
            if layer_path != compressed_path {
                header.set_path(&layer_path)?;
                header.set_size(size);
                header.set_mode(0o644);
                header.set_cksum();
                builder.append(&header, layer_reader).await?;
            }
            layer_sha_vec.push(layer_path);
        }

        // 将config blob 也加入到layer中
        let config_blob = self.target_config_blob_serialize;
        //layer_sha_vec.push(config_blob.digest.sha256.clone());
        let config_blob_path = format!("blobs/sha256/{}", config_blob.digest.sha256);
        write_string_to_builder(config_blob.json_str, config_blob_path.clone(), &mut builder).await?;
        // 写入 index.json

        let manifest_digest = RegDigest::new_with_sha256(bytes_sha256(manifest_raw.as_bytes()));
        let image_index = ImageIndex {
            schema_version: 2,
            manifests: vec![CommonManifestConfig {
                media_type: manifest_media_type.clone(),
                size: manifest_raw.len() as u64,
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
        write_string_to_builder(manifest_raw, manifest_path, &mut builder).await?;
        let mut output = builder.into_inner().await?;
        output.shutdown().await?;
        drop(output);
        temp_path.persist(self.save_path)?;
        Ok(())
    }
}

async fn uncompressed_layer(layer_file: File, compress_type: CompressType, temp_dir: &Path) -> Result<(u64, Box<dyn AsyncRead + Unpin>)> {
    if let CompressType::Tar = compress_type {
        return Ok((layer_file.metadata().await?.len(), Box::new(layer_file)));
    }
    let mut temp_file = File::from_std(tempfile::tempfile_in(temp_dir)?);
    let size = match compress_type {
        CompressType::Tgz => io::copy(&mut GzipDecoder::new(BufReader::new(layer_file)), &mut temp_file).await?,
        CompressType::Zstd => io::copy(&mut ZstdDecoder::new(BufReader::new(layer_file)), &mut temp_file).await?,
        CompressType::Tar => unreachable!(),
    };
    temp_file.seek(SeekFrom::Start(0)).await?;
    Ok((size, Box::new(temp_file)))
}

async fn write_string_to_builder<P: AsRef<Path>>(
    data: String,
    path: P,
    builder: &mut Builder<Box<dyn AsyncWrite + Send + Unpin>>,
) -> Result<()> {
    let size = data.as_bytes().len() as u64;
    let mut header = Header::new_gnu();
    header.set_path(path)?;
    header.set_size(size);
    header.set_mode(0o644);
    header.set_cksum();
    builder.append(&header, data.as_bytes()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::image::oci::{OciConfigBlob, OciManifest};
    use crate::container::manifest::CommonManifestLayer;
    use crate::util::compress::async_compress;
    use tokio::io::AsyncReadExt;

    #[tokio::test]
    async fn archive_reads_uncompressed_layer_from_start() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let input = b"tar payload".repeat(2048);
        for kind in [CompressType::Tgz, CompressType::Zstd] {
            let mut compressed = Vec::new();
            async_compress(kind, &mut input.as_slice(), &mut compressed).await?;
            let path = dir.path().join("layer");
            tokio::fs::write(&path, compressed).await?;
            let (size, mut reader) = uncompressed_layer(File::open(path).await?, kind, dir.path()).await?;
            let mut decoded = Vec::new();
            reader.read_to_end(&mut decoded).await?;
            assert_eq!(size, input.len() as u64);
            assert_eq!(decoded, input);
        }
        Ok(())
    }

    #[tokio::test]
    async fn archive_contains_blobs_named_by_their_digests_and_correct_manifest_size() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let home = HomeDir::new_home_dir(&dir.path().join("cache"))?;
        let uncompressed = b"layer contents".repeat(128);
        let mut compressed = Vec::new();
        async_compress(CompressType::Tgz, &mut uncompressed.as_slice(), &mut compressed).await?;
        let diff_sha = bytes_sha256(&uncompressed);
        let blob_sha = bytes_sha256(&compressed);
        let download_path = dir.path().join("downloaded-layer");
        tokio::fs::write(&download_path, &compressed).await?;
        home.cache.blobs.create_layer_config(&diff_sha, &blob_sha, CompressType::Tgz)?;
        home.cache.blobs.move_to_blob(&download_path, &blob_sha, &diff_sha)?;

        let config_json = serde_json::to_string(&OciConfigBlob::default())?;
        let config_sha = bytes_sha256(config_json.as_bytes());
        let config = ConfigBlobSerialize {
            size: config_json.len() as u64,
            digest: RegDigest::new_with_sha256(config_sha.clone()),
            json_str: config_json.clone(),
        };
        let manifest = OciManifest {
            schema_version: 2,
            media_type: Some(RegContentType::OCI_MANIFEST.val().to_string()),
            config: CommonManifestConfig {
                media_type: RegContentType::OCI_IMAGE_CONFIG.val().to_string(),
                size: config_json.len() as u64,
                digest: format!("sha256:{config_sha}"),
            },
            layers: vec![CommonManifestLayer {
                media_type: RegContentType::OCI_LAYER_TGZ.val().to_string(),
                size: compressed.len() as u64,
                digest: format!("sha256:{blob_sha}"),
            }],
        };
        let manifest_raw = serde_json::to_string(&manifest)?;
        let archive_path = dir.path().join("image.tar");
        TarTargetAdapter {
            image_raw_name: "example:latest".to_string(),
            target_manifest: Manifest::OciV1(manifest),
            target_config_blob_serialize: config,
            save_path: archive_path.clone(),
            use_gzip: false,
        }
        .save_to(&home)
        .await?;

        let unpacked = dir.path().join("unpacked");
        tokio_tar::Archive::new(File::open(&archive_path).await?).unpack(&unpacked).await?;
        let index: ImageIndex = serde_json::from_slice(&tokio::fs::read(unpacked.join("index.json")).await?)?;
        assert_eq!(index.manifests[0].size, manifest_raw.len() as u64);
        assert_eq!(tokio::fs::read(unpacked.join("blobs/sha256").join(blob_sha)).await?, compressed);
        assert_eq!(
            tokio::fs::read(unpacked.join("blobs/sha256").join(diff_sha.clone())).await?,
            uncompressed
        );
        assert_eq!(
            tokio::fs::read(unpacked.join("blobs/sha256").join(config_sha)).await?,
            config_json.as_bytes()
        );
        assert_eq!(
            tokio::fs::read(unpacked.join("blobs/sha256").join(bytes_sha256(manifest_raw.as_bytes()))).await?,
            manifest_raw.as_bytes()
        );
        let docker_manifest: Vec<TarManifestJson> = serde_json::from_slice(&tokio::fs::read(unpacked.join("manifest.json")).await?)?;
        assert_eq!(docker_manifest[0].layers, vec![format!("blobs/sha256/{diff_sha}")]);
        Ok(())
    }
}
