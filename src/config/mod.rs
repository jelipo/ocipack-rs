use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::path::{Path, PathBuf};

use anyhow::Result;
use home::home_dir;

use crate::config::userconfig::{UserDockerConfig, UserDockerConfigAuth};
use crate::reg::http::RegistryAuth;


pub mod dockerfile;
pub mod ocifile;
pub mod userconfig;

pub struct BaseImage {
    /// registry的host地址
    pub reg_host: String,
    /// image的名称
    pub image_name: String,
    /// 可以是TAG或者digest
    pub reference: String,
    /// 验证方式
    pub auth_type: RegAuthType,
}

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
}

fn read_docker_config(config_path: PathBuf, reg_host: &str) -> Result<Option<RegistryAuth>> {
    if config_path.is_file() {
        let config_file = File::open(config_path)?;
        let user_docker_config = serde_json::from_reader::<_, UserDockerConfig>(config_file)?;
        Ok(match user_docker_config.auths.get(reg_host) {
            None => None,
            Some(auth) => match &auth.auth {
                None => None,
                Some(base64_str) => {
                    let vec = base64::decode(base64_str)?;
                    let decode_str = String::from_utf8(vec)?;
                    let mut split = decode_str.split(":");
                    let username = split.next().expect("error docker file").to_string();
                    let password = split.next().expect("error docker file").to_string();
                    Some(RegistryAuth {
                        username,
                        password,
                    })
                }
            }
        })
    } else {
        Ok(None)
    }
}
