use std::time::Duration;

use anyhow::Result;
use reqwest::{Method, Url};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use serde::Serialize;

#[derive(Clone)]
pub struct HttpClient {
    registry_addr: String,
    username: String,
    password: String,
    client: Client,
}


impl HttpClient {
    pub fn new(registry_addr: String, username: &str, password: &str) -> Result<HttpClient> {
        let client = reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .gzip(true)
            .connect_timeout(Duration::from_secs(5))
            .danger_accept_invalid_certs(true)
            .deflate(true)
            .redirect(Policy::default())
            .build()?;
        Ok(HttpClient {
            registry_addr,
            username: username.to_string(),
            password: password.to_string(),
            client,
        })
    }

    pub fn request<T: Serialize + ?Sized>(
        &self, path: &str, method: Method, body: Option<&T>,
    ) -> Result<String> {
        let url = Url::parse((self.registry_addr.clone() + path).as_str())?;
        let mut builder = self.client.request(method, url)
            .basic_auth(self.username.clone(), Some(self.password.clone()));
        if let Some(body_o) = body {
            builder = builder.json(body_o)
        }
        let request = builder.build()?;
        let response = self.client.execute(request)?;
        let string = String::from_utf8(response.bytes()?.to_vec())?;
        Ok(string)
    }
}