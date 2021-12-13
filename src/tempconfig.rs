use serde::Deserialize;

#[derive(Deserialize)]
pub struct TempConfig {
    pub from: FromConfig,
    pub to: ToConfig,
    pub home_dir: String,
    pub test_file: String,
}

#[derive(Deserialize)]
pub struct FromConfig {
    pub registry: String,
    pub image_name: String,
    pub reference: String,
    pub username: String,
    pub password: String,

}

#[derive(Deserialize)]
pub struct ToConfig {
    pub registry: String,
    pub image_name: String,
    pub reference: String,
    pub username: String,
    pub password: String,
}
