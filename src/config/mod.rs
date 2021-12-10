use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;

use anyhow::Result;

use crate::RegistryAuth;

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
}

pub enum RegAuthType {
    LocalDockerAuth { reg_host: String },
    CustomPassword { username: String, password: String },
}

impl RegAuthType {
    pub fn get_auth(self) -> Result<Option<RegistryAuth>> {
        match self {
            RegAuthType::LocalDockerAuth { reg_host } => {
                Ok(Some(RegistryAuth {
                    username: "".to_string(),
                    password: "".to_string(),
                }))
            }
            RegAuthType::CustomPassword { username, password } => Ok(Some(RegistryAuth {
                username,
                password,
            }))
        }
    }
}

