use std::io::Read;
use std::option::Option::Some;
use std::time::Duration;

use anyhow::{Error, Result};
use bytes::buf::Reader;
use bytes::{Buf, Bytes};
use log::{debug, warn};
use reqwest::blocking::{Client, Request, Response};
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::redirect::Policy;
use reqwest::{Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::digest::DynDigest;
use sha2::{Digest, Sha256};

#[derive(Clone)]
pub struct RegistryHttpClient {
    registry_addr: String,
    username: String,
    password: String,
    client: Client,
}

impl RegistryHttpClient {
    pub fn new(reg_addr: String, username: &str, password: &str) -> Result<RegistryHttpClient> {
        let client = reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(10))
            .gzip(true)
            .connect_timeout(Duration::from_secs(5))
            .danger_accept_invalid_certs(true)
            .deflate(true)
            .redirect(Policy::default())
            .build()?;
        Ok(RegistryHttpClient {
            registry_addr: reg_addr,
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
        let success_response = self.do_request(path, method, body)?;
        let string1 = success_response.docker_content_digest()?;
        let bytes = success_response.bytes_body()?;
        let vec = bytes.to_vec();
        let mut hasher = Sha256::new();
        DynDigest::update(&mut hasher, &vec[..]);
        let x = &hasher.finalize()[..];
        Err(Error::msg("dsa"))
    }

    fn do_request<T: Serialize + ?Sized>(
        &self,
        path: &str,
        method: Method,
        body: Option<&T>,
    ) -> Result<RegistryResponse> {
        let request = self.build_request(path, method, body)?;
        let mut http_response = self.client.execute(request)?;
        let response = RegistryResponse::new_registry_response(http_response)?;
        return if response.is_success() {
            Ok(response)
        } else {
            let content_type = response.get_content_type();
            Err(Error::msg(format!(
                "Request to registry failed,status_code:{} ,content-type:{} ,body:{}",
                response.status_code().as_str(),
                content_type,
                response.body_str()?,
            )))
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

pub struct RegistryErr {
    errors: Vec<SingleRegistryError>,
}

pub struct SingleRegistryError {
    code: String,
    message: String,
}

pub struct RegistryResponse {
    http_response: Response,
    content_type: String,
}

impl RegistryResponse {
    pub fn new_registry_response(http_response: Response) -> Result<RegistryResponse> {
        let headers = http_response.headers();
        let content_type_value = headers.get("content-type").ok_or(Error::msg(format!(
            "Request to registry failed,status_code:{}",
            http_response.status().as_str(),
        )))?;
        let string = String::from(content_type_value.to_str()?);
        Ok(RegistryResponse {
            http_response,
            content_type: string,
        })
    }

    pub fn get_content_type(&self) -> String {
        self.content_type.clone()
    }

    pub fn is_success(&self) -> bool {
        self.http_response.status().is_success()
    }

    pub fn status_code(&self) -> StatusCode {
        self.http_response.status()
    }

    pub fn body_str(self) -> Result<String> {
        Ok(self.http_response.text()?)
    }

    pub fn json_body<R: DeserializeOwned>(self) -> Result<R> {
        if self.content_type.contains("json") {
            Ok(self.http_response.json::<R>()?)
        } else {
            Err(Error::msg("Content-type not contains json"))
        }
    }

    pub fn bytes_body(self) -> Result<Bytes> {
        Ok(self.http_response.bytes()?)
    }

    pub fn docker_content_digest(&self) -> Result<String> {
        return match self.http_response.headers().get("Docker-Content-Digest") {
            None => Err(Error::msg("header 'Docker-Content-Digest' not found")),
            Some(value) => Ok(value.to_str()?.to_string()),
        };
    }
}
