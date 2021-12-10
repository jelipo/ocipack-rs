use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDockerConfig {
    auths: HashMap<String, UserDockerConfigAuth>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserDockerConfigAuth {
    auth: String,
}