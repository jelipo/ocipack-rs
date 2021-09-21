use std::time::Duration;

use anyhow::Result;
use reqwest::{Method, Request, RequestBuilder, Url};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;

#[derive(Clone)]
pub struct HttpClient {
    registry_addr: String,
    client: Client,
}


impl HttpClient {
    pub fn new(registry_addr: String) -> Result<HttpClient> {
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
            client,
        })
    }

    pub fn request(&self) -> Result<String> {
        let builder = self.client.request(Method::GET, Url::parse("https://me.jelipo.com/")?);
        let request = builder.build()?;
        let response = self.client.execute(request)?;
        let string = String::from_utf8(response.bytes()?.to_vec())?;
        Ok(string)
    }
}