use std::time::Duration;

use anyhow::Result;
use reqwest::blocking::Client;
use reqwest::redirect::Policy;


pub struct HttpClient {
    registry_addr: String,
    http_client: Client,
}


impl HttpClient {
    pub fn new_client(registry_addr: String) -> Result<HttpClient> {
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
            http_client: client,
        })
    }
}