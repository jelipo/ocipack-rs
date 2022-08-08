use std::path::PathBuf;

use anyhow::{anyhow, Result};
use bytes::Buf;
use home::home_dir;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio_util::codec::{BytesCodec, FramedRead};
use ubyte::ToByteUnit;

use crate::config::cmd::BaseAuth;
use crate::config::userconfig::UserDockerConfig;
use crate::const_data::{DEFAULT_IMAGE_HOST, DEFAULT_IMAGE_HUB_URI};
use crate::reg::http::RegistryAuth;

pub mod cmd;
pub mod global;
pub mod userconfig;

#[derive(Clone)]
pub enum RegAuthType {
    LocalDockerAuth { reg_host: String },
    CustomPassword { username: String, password: String },
}

impl RegAuthType {
    pub async fn get_auth(self) -> Result<Option<RegistryAuth>> {
        match self {
            RegAuthType::LocalDockerAuth { reg_host } => Ok(match home_dir() {
                None => None,
                Some(dir) => read_docker_config(dir.join(".docker/config.json"), &reg_host).await?,
            }),
            RegAuthType::CustomPassword { username, password } => Ok(Some(RegistryAuth { username, password })),
        }
    }

    pub fn build_auth(image_host: String, base_auth: Option<&BaseAuth>) -> RegAuthType {
        match base_auth.as_ref() {
            None => RegAuthType::LocalDockerAuth {
                reg_host: if image_host.eq(DEFAULT_IMAGE_HOST) {
                    image_host
                } else {
                    DEFAULT_IMAGE_HUB_URI.to_string()
                },
            },
            Some(auth) => RegAuthType::CustomPassword {
                username: auth.username.clone(),
                password: auth.password.clone(),
            },
        }
    }
}

async fn read_docker_config(config_path: PathBuf, reg_host: &str) -> Result<Option<RegistryAuth>> {
    if config_path.is_file() {
        let mut config_file = File::open(config_path).await?;
        let file_size = config_file.metadata().await?.len();
        let mut file_bytes = vec![0u8; file_size as usize];
        config_file.read_to_end(&mut file_bytes).await?;
        let user_docker_config = serde_json::from_slice::<UserDockerConfig>(&file_bytes)?;
        get_auth_from_dockerconfig(user_docker_config, reg_host)
    } else {
        Ok(None)
    }
}

fn get_auth_from_dockerconfig(user_docker_config: UserDockerConfig, reg_host: &str) -> Result<Option<RegistryAuth>> {
    if let Some(auth_map) = user_docker_config.auths {
        if let Some(auth) = auth_map.get(reg_host) {
            if let Some(base64_str) = &auth.auth {
                let vec = base64::decode(base64_str)?;
                let decode_str = String::from_utf8(vec)?;
                let mut split = decode_str.split(':');
                let username = split.next().ok_or_else(|| anyhow!("error docker file"))?.to_string();
                let password = split.next().ok_or_else(|| anyhow!("error docker file"))?.to_string();
                return Ok(Some(RegistryAuth { username, password }));
            }
        }
    }
    Ok(None)
}
