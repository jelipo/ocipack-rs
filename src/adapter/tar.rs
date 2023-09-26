use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Result;
use flate2::Compression;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use tar::Builder;

use crate::adapter::{TargetImageAdapter, TargetInfo};
use crate::container::ConfigBlobSerialize;
use crate::container::manifest::{CommonManifestConfig, Manifest};

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
        let file = File::create(self.save_path)?;
        let write_box = if self.use_gzip {
            let encoder = GzEncoder::new(file, Compression::fast());
            Box::new(encoder) as Box<dyn Write>
        } else {
            Box::new(file) as Box<dyn Write>
        };
        let builder = Builder::new(write_box);

        // gen image index
        let manifest_config = match self.target_manifest {
            Manifest::OciV1(oci) => oci.config.clone(),
            Manifest::DockerV2S2(dockerv2s2) => dockerv2s2.config.clone(),
        };
        let image_index = ImageIndex {
            schema_version: 2,
            manifests: vec![manifest_config],
        };
        //
        TarManifestJson {
            config: format!("blobs/sha256/{}", self.target_config_blob_serialize.digest.sha256),
            repo_tags: vec![],
            layers: vec![],
        };
        Ok(())
    }
}