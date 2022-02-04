use anyhow::Error;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;

use crate::reg::{ConfigBlobEnum, ConfigBlobSerialize, Layer, LayerConvert, Reference, RegContentType, RegDigest};
use crate::reg::docker::DockerManifest;
use crate::reg::oci::OciManifest;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommonManifestLayer {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CommonManifestConfig {
    pub media_type: String,
    pub size: u64,
    pub digest: String,
}

#[derive(Clone)]
pub enum Manifest {
    OciV1(OciManifest),
    DockerV2S2(DockerManifest),
}

impl Manifest {
    pub fn to_oci_v1(self, config_blob_serialize: &ConfigBlobSerialize) -> anyhow::Result<OciManifest> {
        Ok(match self {
            Manifest::OciV1(mut oci) => {
                set_config_blob(&mut oci.config, config_blob_serialize);
                oci
            }
            Manifest::DockerV2S2(mut docker) => OciManifest {
                schema_version: 2,
                media_type: Some(RegContentType::OCI_MANIFEST.val().to_string()),
                config: {
                    set_config_blob(&mut docker.config, config_blob_serialize);
                    docker.config
                },
                layers: docker.layers.into_iter().map(|mut common_layer| {
                    common_layer.media_type = dockerv2s2_to_ociv1(&common_layer.media_type)?;
                    Ok(common_layer)
                }).collect::<anyhow::Result<Vec<CommonManifestLayer>>>()?,
            }
        })
    }

    pub fn to_docker_v2_s2(self, config_blob_serialize: &ConfigBlobSerialize) -> anyhow::Result<DockerManifest> {
        Ok(match self {
            Manifest::OciV1(mut oci) => DockerManifest {
                schema_version: 2,
                media_type: RegContentType::DOCKER_MANIFEST.val().to_string(),
                config: {
                    set_config_blob(&mut oci.config, config_blob_serialize);
                    oci.config
                },
                layers: oci.layers.into_iter().map(|mut common_layer| {
                    common_layer.media_type = ociv1_to_dockerv2s2(&common_layer.media_type)?;
                    Ok(common_layer)
                }).collect::<anyhow::Result<Vec<CommonManifestLayer>>>()?,
            },
            Manifest::DockerV2S2(mut docker) => {
                set_config_blob(&mut docker.config, config_blob_serialize);
                docker
            },
        })
    }

    pub fn layers(&self) -> Vec<Layer> {
        match &self {
            Manifest::OciV1(oci) => oci.get_layers(),
            Manifest::DockerV2S2(docker) => docker.get_layers(),
        }
    }

    pub fn add_top_gz_layer(&mut self, size: u64, tgz_sha256: String) {
        let reg_digest = RegDigest::new_with_sha256(tgz_sha256);
        match self {
            Manifest::OciV1(oci) => oci.layers.insert(0, CommonManifestLayer {
                media_type: RegContentType::DOCKER_FOREIGN_LAYER_TGZ.val().to_string(),
                size,
                digest: reg_digest.digest,
            }),
            Manifest::DockerV2S2(docker) => docker.layers.insert(0, CommonManifestLayer {
                media_type: RegContentType::OCI_LAYER_NONDISTRIBUTABLE_TGZ.val().to_string(),
                size,
                digest: reg_digest.digest,
            }),
        }
    }

    pub fn config_digest(&self) -> &str {
        match self {
            Manifest::OciV1(oci) => &oci.config.digest,
            Manifest::DockerV2S2(docker) => &docker.config.digest
        }
    }
}

pub fn ociv1_to_dockerv2s2(media_type: &str) -> Result<String> {
    let new_media_type = if media_type == RegContentType::OCI_LAYER_TGZ.val() {
        RegContentType::DOCKER_LAYER_TGZ
    } else if media_type == RegContentType::OCI_LAYER_NONDISTRIBUTABLE_TGZ.val() {
        RegContentType::DOCKER_FOREIGN_LAYER_TGZ
    } else if media_type == RegContentType::OCI_LAYER_TAR.val()
        || media_type == RegContentType::OCI_LAYER_NONDISTRIBUTABLE_TAR.val() {
        return Err(Error::msg(format!("docker not support tar layer,source type:{}", media_type)));
    } else {
        return Err(Error::msg(format!("error oci layer type:{}", media_type)));
    };
    Ok(new_media_type.val().to_string())
}

pub fn dockerv2s2_to_ociv1(media_type: &str) -> Result<String> {
    let new_media_type = if media_type == RegContentType::DOCKER_LAYER_TGZ.val() {
        RegContentType::OCI_LAYER_TGZ
    } else if media_type == RegContentType::DOCKER_FOREIGN_LAYER_TGZ.val() {
        RegContentType::OCI_LAYER_NONDISTRIBUTABLE_TGZ
    } else {
        return Err(Error::msg(format!("error docker layer type:{}", media_type)));
    };
    Ok(new_media_type.val().to_string())
}


fn set_config_blob(common_config: &mut CommonManifestConfig, config_blob_serialize: &ConfigBlobSerialize) {
    common_config.media_type = RegContentType::DOCKER_CONTAINER_IMAGE.val().to_string();
    common_config.digest = config_blob_serialize.digest.digest.clone();
    common_config.size = config_blob_serialize.size;
}