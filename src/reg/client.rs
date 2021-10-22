use std::io::Read;
use std::option::Option::Some;
use std::time::Duration;

use anyhow::{Error, Result};
use log::{debug, warn};
use reqwest::blocking::{Client, Request};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{Method, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;

const DOCKER_JSON_CONTENT_TYPE: &str = "application/vnd.docker.distribution.manifest.v2+json";

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

    pub fn request_registry<T: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        path: &str,
        method: Method,
        body: Option<&T>,
    ) -> Result<R> {
        return self.do_request::<T, R>(path, method, body);
    }

    fn do_request<T: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        path: &str,
        method: Method,
        body: Option<&T>,
    ) -> Result<R> {
        let request = self.build_request(path, method, body)?;
        let mut response = self.client.execute(request)?;
        let status_code = response.status();
        let headers = response.headers();
        return if status_code.is_success() {
            Ok(response.json::<R>()?)
        } else {
            match get_content_type(headers) {
                None => Err(Error::msg(format!(
                    "Request to registry failed,status_code:{}",
                    status_code.as_str()
                ))),
                Some(content_type) => {
                    let mut body = String::default();
                    let i = response.read_to_string(&mut body)?;
                    Err(Error::msg(format!(
                        "Request to registry failed,status_code:{} ,content-type:{} ,body:{}",
                        status_code.as_str(),
                        content_type,
                        body,
                    )))
                }
            }
        };
    }

    fn build_request<T: Serialize + ?Sized>(
        &self,
        path: &str,
        method: Method,
        body: Option<&T>,
    ) -> Result<Request> {
        let url = Url::parse((self.registry_addr.clone() + path).as_str())?;
        let mut builder = self
            .client
            .request(method, url)
            .basic_auth(&self.username, Some(&self.password));
        if let Some(body_o) = body {
            builder = builder.json::<T>(body_o)
        }
        Ok(builder.build()?)
    }
}

fn get_content_type(headers: &HeaderMap<HeaderValue>) -> Option<&str> {
    return if let Some(Ok(content_type)) = headers.get("content-type").map(|value| value.to_str()) {
        Some(content_type)
    } else {
        None
    };
}

pub enum ContentType {
    /// application/vnd.docker.distribution.manifest.v2+json
    ApplicationVndDockerDistributionManifestV2Json,
}

pub struct RegistryErr {
    errors: Vec<SingleRegistryError>,
}

pub struct SingleRegistryError {
    code: String,
    message: String,
}
