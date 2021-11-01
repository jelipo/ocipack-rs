use reqwest::header::HeaderMap;

pub mod image;
pub mod registry;
pub mod home;
pub mod http;

pub struct Reference<'a> {
    /// Image的名称
    pub image_name: &'a str,
    /// 可以是TAG或者digest
    pub reference: &'a str,
}

