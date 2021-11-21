use std::io::Read;
use std::option::Option::Some;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Error, Result};
use bytes::Bytes;
use reqwest::{Method, StatusCode};
use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use serde::de::DeserializeOwned;
use serde::Serialize;
use crate::reg::BlobDownConfig;

use crate::reg::docker::http::{do_request_raw, get_header, HttpAuth, RegistryAccept, RegistryAuth};
use crate::reg::docker::http::auth::RegTokenHandler;
use crate::reg::docker::http::download::RegDownloader;
use crate::util::sha;

pub struct RegistryHttpClient {
    registry_addr: String,
    client: Client,
    basic_auth: Option<HttpAuth>,
    reg_token_handler: RegTokenHandler,
}

impl RegistryHttpClient {
    pub fn new(reg_addr: String, auth: Option<RegistryAuth>) -> Result<RegistryHttpClient> {
        let client = reqwest::blocking::ClientBuilder::new()
            .timeout(Duration::from_secs(30))
            .gzip(true)
            .connect_timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .deflate(true)
            .redirect(Policy::default())
            .build()?;
        let http_auth_opt = auth.map(|reg_auth| HttpAuth::BasicAuth {
            username: reg_auth.username,
            password: reg_auth.password,
        });
        Ok(RegistryHttpClient {
            registry_addr: reg_addr.clone(),
            client: client.clone(),
            basic_auth: http_auth_opt.clone(),
            reg_token_handler: RegTokenHandler::new_reg_token_handler(
                reg_addr,
                http_auth_opt,
                client,
            ),
        })
    }

    pub fn request_registry<T: Serialize + ?Sized, R: DeserializeOwned>(
        &mut self, path: &str, scope: &Option<&str>, method: Method,
        accept: &Option<RegistryAccept>, body: Option<&T>,
    ) -> Result<R> {
        let success_response = self.do_request(path, scope, method, accept, body)?;
        let header_docker_content_digest = success_response
            .header_docker_content_digest()
            .expect("No Docker-Content-Digest header");
        let body_bytes = success_response.bytes_body();
        let body_sha256 = format!("sha256:{}", sha::sha256(body_bytes));
        // if body_bytes.len() != 0 && body_sha256 != header_docker_content_digest {
        //     return Err(Error::msg("docker_content_digest verification failed"));
        // }
        success_response.json_body::<R>()
    }

    pub fn head_request_registry(&mut self, path: &str, scope: &Option<&str>) -> Result<SimpleRegistryResponse> {
        let http_response = self.do_request_raw::<u8>(path, scope, Method::HEAD, &None, None)?;
        Ok(SimpleRegistryResponse {
            status_code: http_response.status(),
        })
    }

    fn do_request_raw<T: Serialize + ?Sized>(
        &mut self, path: &str, scope: &Option<&str>, method: Method,
        accept: &Option<RegistryAccept>, body: Option<&T>,
    ) -> Result<Response> {
        let url = self.registry_addr.clone() + path;
        let token = self.reg_token_handler.token(scope)?;
        let auth = Some(HttpAuth::BearerToken { token });
        let http_response = do_request_raw(&self.client, url.as_str(), method, &auth, accept, body)?;
        Ok(http_response)
    }

    fn do_request<T: Serialize + ?Sized>(
        &mut self, path: &str, scope: &Option<&str>, method: Method,
        accept: &Option<RegistryAccept>, body: Option<&T>,
    ) -> Result<FullRegistryResponse> {
        let http_response = self.do_request_raw(path, scope, method, accept, body)?;
        let response = FullRegistryResponse::new_registry_response(http_response)?;
        return if response.is_success() {
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

    pub fn download(&mut self, path: &str, blob_down_config: BlobDownConfig, scope: &str) -> Result<RegDownloader> {
        let url = format!("{}{}", &self.registry_addr, path);
        let token = self.reg_token_handler.token(&Some(scope))?;
        let downloader = RegDownloader::new_reg_downloader(
            url,
            Some(HttpAuth::BearerToken { token }),
            self.client.clone(),
            blob_down_config,
        )?;
        Ok(downloader)
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
        Ok(FullRegistryResponse {
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
