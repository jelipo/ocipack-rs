use std::io::Read;
use std::option::Option::Some;
use std::time::Duration;

use anyhow::{Error, Result};
use bytes::buf::Reader;
use bytes::{Buf, Bytes};
use log::{debug, warn};
use reqwest::blocking::{Client, Request, Response};
use reqwest::header::{HeaderMap, HeaderValue, ToStrError};
use reqwest::redirect::Policy;
use reqwest::{Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sha2::digest::DynDigest;
use sha2::{Digest, Sha256};

use crate::util::sha;

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
        let header_docker_content_digest = success_response
            .header_docker_content_digest()
            .expect("No Docker-Content-Digest header");
        let body_bytes = success_response.bytes_body();
        let body_sha256 = format!("sha256:{}", body_sha256(body_bytes));
        if body_bytes.len() != 0 && body_sha256 != header_docker_content_digest {
            return Err(Error::msg("docker_content_digest verification failed"));
        }
        success_response.json_body::<R>()
    }

    pub fn head_request_registry(&self, path: &str) -> SimpleRegistryResponse {
        let http_response = self.do_request_raw(path, Method::HEAD, body)?;
        SimpleRegistryResponse {
            status_code: http_response.status(),
        }
    }

    fn do_request_raw<T: Serialize + ?Sized>(
        &self,
        path: &str,
        method: Method,
        body: Option<&T>,
    ) -> Result<Response> {
        let request = self.build_request(path, method, body)?;
        let mut http_response = self.client.execute(request)?;
        Ok(http_response)
    }

    fn do_request<T: Serialize + ?Sized>(
        &self,
        path: &str,
        method: Method,
        body: Option<&T>,
    ) -> Result<FullRegistryResponse> {
        let http_response = self.do_request_raw(path, method, body)?;
        return if http_response.status().is_success() {
            let response = RegistryResponse::new_registry_response(http_response)?;
            Ok(response)
        } else {
            match response.get_content_type() {
                None => Err(Error::msg(format!(
                    "Request to registry failed,status_code:{}",
                    response.status_code().as_str()
                ))),
                Some(content_type) => Err(Error::msg(format!(
                    "Request to registry failed,status_code:{} ,content-type:{} ,body:{}",
                    response.status_code().as_str(),
                    content_type,
                    response.body_str(),
                ))),
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
        let mut builder =
            self.client.request(method, url).basic_auth(&self.username, Some(&self.password));
        if let Some(body_o) = body {
            builder = builder.json::<T>(body_o)
        }
        Ok(builder.build()?)
    }
}

pub struct FullRegistryResponse {
    body_bytes: Bytes,
    content_type: Option<String>,
    docker_content_digest: Option<String>,
    http_status: StatusCode,
}

/// Registry的Response包装
impl FullRegistryResponse {
    pub fn new_registry_response(http_response: Response) -> Result<FullRegistryResponse> {
        let headers = http_response.headers();
        let content_type_opt = get_header(headers, "content-type");
        let docker_content_digest_opt = get_header(headers, "Docker-Content-Digest");
        let code = http_response.status();
        let body_bytes = http_response.bytes()?;
        Ok(RegistryResponse {
            body_bytes,
            content_type: content_type_opt,
            docker_content_digest: docker_content_digest_opt,
            http_status: code,
        })
    }

    pub fn get_content_type(&self) -> Option<&str> {
        self.content_type.as_ref().map(|str| str.as_str())
    }

    pub fn is_success(&self) -> bool {
        self.status_code().is_success()
    }

    pub fn status_code(&self) -> &StatusCode {
        &self.http_status
    }

    pub fn body_str(&self) -> String {
        String::from_utf8_lossy(&self.body_bytes[..]).into()
    }

    pub fn bytes_body(&self) -> &Bytes {
        &self.body_bytes
    }

    pub fn json_body<R: DeserializeOwned>(&self) -> Result<R> {
        let json_result = serde_json::from_slice::<R>(self.body_bytes.as_ref());
        Ok(json_result?)
    }

    pub fn header_docker_content_digest(&self) -> Option<String> {
        self.docker_content_digest.clone()
    }
}

pub trait RegistryResponse {
    fn success(&self) -> bool;

    fn status_code(&self) -> u16;
}

/// 一个简单的Registry的Response，只包含状态码
pub struct SimpleRegistryResponse {
    status_code: StatusCode,
}

impl RegistryResponse for SimpleRegistryResponse {
    fn success(&self) -> bool {
        self.status_code.is_success()
    }

    fn status_code(&self) -> u16 {
        self.status_code.as_u16()
    }
}

fn get_header(headers: &HeaderMap, header_name: &str) -> Option<String> {
    headers.get(header_name).and_then(|value| match value.to_str() {
        Ok(str) => Some(String::from(str)),
        Err(_) => None,
    })
}
