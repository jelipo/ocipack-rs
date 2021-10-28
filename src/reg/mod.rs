use reqwest::header::HeaderMap;

pub mod client;
mod download;
pub mod image;
pub mod registry;
mod home;

pub struct Reference<'a> {
    /// Image的名称
    pub image_name: &'a str,
    /// 可以是TAG或者digest
    pub reference: &'a str,
}

fn get_header(headers: &HeaderMap, header_name: &str) -> Option<String> {
    headers.get(header_name).and_then(|value| match value.to_str() {
        Ok(str) => Some(String::from(str)),
        Err(_) => None,
    })
}