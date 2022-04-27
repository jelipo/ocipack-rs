use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDockerConfig {
    pub auths: Option<HashMap<String, UserDockerConfigAuth>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDockerConfigAuth {
    pub auth: Option<String>,
}
