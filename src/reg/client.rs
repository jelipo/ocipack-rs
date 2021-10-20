use std::time::Duration;

use anyhow::{Error, Result};
use reqwest::{Method, Url};
use reqwest::blocking::{Client, Request};
use reqwest::redirect::Policy;
use serde::de::DeserializeOwned;
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

    pub fn request<T: Serialize + ?Sized, R: DeserializeOwned>(
        &self, path: &str, method: Method, body: Option<&T>,
    ) -> Result<R> {
        let url = Url::parse((self.registry_addr.clone() + path).as_str())?;
        let request = self.build_request(url.to_string(), method, body)?;
        let response = self.client.execute(request)?;
        return if response.status().is_success() {
            Ok(response.json::<R>()?)
        } else {
            Err(Error::msg("Request to image registry failed."))
        };
    }

    fn build_request<T: Serialize + ?Sized>(&self, url: String, method: Method, body: Option<&T>) -> Result<Request> {
        let mut builder = self.client.request(method, url)
            .basic_auth(&self.username, Some(&self.password));
        if let Some(body_o) = body {
            builder = builder.json(body_o)
        }
        Ok(builder.build()?)
    }
}