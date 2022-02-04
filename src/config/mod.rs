use std::fs::File;
use std::path::PathBuf;

use anyhow::Result;
use home::home_dir;

use crate::config::cmd::BaseAuth;
use crate::config::userconfig::UserDockerConfig;
use crate::reg::http::RegistryAuth;

pub mod userconfig;
pub mod global;
pub mod cmd;

pub struct BaseImage {
    pub use_https: bool,
    /// registry的host地址
    pub reg_host: String,
    /// image的名称
    pub image_name: String,
    /// 可以是TAG或者digest
    pub reference: String,
    /// 验证方式
    pub auth_type: RegAuthType,
}

#[derive(Clone)]
pub enum RegAuthType {
    LocalDockerAuth { reg_host: String },
    CustomPassword { username: String, password: String },
}

impl RegAuthType {
    pub fn get_auth(self) -> Result<Option<RegistryAuth>> {
        match self {
            RegAuthType::LocalDockerAuth { reg_host } => {
                Ok(match home_dir() {
                    None => None,
                    Some(dir) => read_docker_config(dir.join(".docker/config.json"), &reg_host)?
                })
            }
            RegAuthType::CustomPassword { username, password } => Ok(Some(RegistryAuth {
                username,
                password,
            }))
        }
    }

    pub fn build_auth(image_host: Option<&String>, base_auth: Option<&BaseAuth>) -> RegAuthType {
        match base_auth.as_ref() {
            None => RegAuthType::LocalDockerAuth {
                reg_host: image_host.map(|s| s.as_str())
                    .unwrap_or("https://index.docker.io/v1/").to_string()
            },
            Some(auth) => RegAuthType::CustomPassword {
                username: auth.username.clone(),
                password: auth.password.clone(),
            }
        }
    }
}

fn read_docker_config(config_path: PathBuf, reg_host: &str) -> Result<Option<RegistryAuth>> {
    if config_path.is_file() {
        let config_file = File::open(config_path)?;
        let user_docker_config = serde_json::from_reader::<_, UserDockerConfig>(config_file)?;
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
                let username = split.next().expect("error docker file").to_string();
                let password = split.next().expect("error docker file").to_string();
                return Ok(Some(RegistryAuth {
                    username,
                    password,
                }));
            }
        }
    }
    return Ok(None);
}