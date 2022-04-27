use std::io::Read;
use std::option::Option::Some;
use std::path::Path;
use std::time::Duration;

use anyhow::{Error, Result};
use bytes::Bytes;
use reqwest::blocking::{Client, Response};
use reqwest::redirect::Policy;
use reqwest::{Method, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::reg::http::auth::{RegTokenHandler, TokenType};
use crate::reg::http::download::RegDownloader;
use crate::reg::http::upload::RegUploader;
use crate::reg::http::{do_request_raw, get_header, HttpAuth, RegistryAuth};
use crate::reg::{BlobConfig, RegContentType};
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
            reg_token_handler: RegTokenHandler::new_reg_token_handler(reg_addr, http_auth_opt, client),
        })
    }

    pub fn request_registry_body<T: Serialize + ?Sized, R: DeserializeOwned>(&mut self, request: ClientRequest<T>) -> Result<R> {
        let success_response = self.request_full_response(request)?;
        let body_bytes = success_response.bytes_body();
        let _body_sha256 = format!("sha256:{}", sha::sha256(body_bytes));
        success_response.json_body::<R>()
    }

    pub fn request_full_response<T: Serialize + ?Sized>(&mut self, request: ClientRequest<T>) -> Result<FullRegistryResponse> {
        self.do_request(request)
    }

    pub fn simple_request<T: Serialize + ?Sized>(&mut self, request: ClientRequest<T>) -> Result<RawRegistryResponse> {
        let http_response = self.do_request_raw(request)?;
        Ok(RawRegistryResponse { response: http_response })
    }

    fn do_request_raw<B: Serialize + ?Sized>(&mut self, request: ClientRequest<B>) -> Result<Response> {
        let url = self.registry_addr.clone() + request.path;
        let token = self.reg_token_handler.token(request.scope, request.token_type)?;
        let auth = Some(HttpAuth::BearerToken { token });
        let http_response = do_request_raw(
            &self.client,
            url.as_str(),
            request.method,
            auth.as_ref(),
            request.accept,
            request.body,
            request.request_content_type,
        )?;
        Ok(http_response)
    }

    fn do_request<T: Serialize + ?Sized>(&mut self, request: ClientRequest<T>) -> Result<FullRegistryResponse> {
        let http_response = self.do_request_raw(request)?;
        let response = FullRegistryResponse::new_registry_response(http_response)?;
        return if response.is_success() {
            Ok(response)
        } else {
            Err(Error::msg(match response.get_content_type() {
                None => format!("Request to registry failed,status_code:{}", response.status_code().as_str()),
                Some(content_type) => format!(
                    "Request to registry failed,status_code:{} ,content-type:{} ,body:{}",
                    response.status_code().as_str(),
                    content_type,
                    response.body_str()
                ),
            }))
        };
    }

    pub fn download(
        &mut self,
        path: &str,
        blob_down_config: BlobConfig,
        scope: &str,
        layer_size: Option<u64>,
    ) -> Result<RegDownloader> {
        let url = format!("{}{}", &self.registry_addr, path);
        let token = self.reg_token_handler.token(Some(scope), TokenType::Pull)?;
        let downloader = RegDownloader::new_reg(
            url,
            Some(HttpAuth::BearerToken { token }),
            self.client.clone(),
            blob_down_config,
            layer_size,
        )?;
        Ok(downloader)
    }

    pub fn upload(&mut self, url: String, blob_config: BlobConfig, scope: &str, file_local_path: &Path) -> Result<RegUploader> {
        let token = self.reg_token_handler.token(Some(scope), TokenType::PushAndPull)?;
        Ok(RegUploader::new_uploader(
            url,
            HttpAuth::BearerToken { token },
            self.client.clone(),
            blob_config,
            file_local_path.metadata()?.len(),
        ))
    }
}

pub struct FullRegistryResponse {
    body_bytes: Bytes,
    content_type: Option<String>,
    docker_content_digest: Option<String>,
    location_header: Option<String>,
    http_status: StatusCode,
}

/// Registry的Response包装
impl FullRegistryResponse {
    pub fn new_registry_response(http_response: Response) -> Result<FullRegistryResponse> {
        let headers = http_response.headers();
        let content_type_opt = get_header(headers, "content-type");
        let docker_content_digest_opt = get_header(headers, "Docker-Content-Digest");
        let location_header = get_header(headers, "Location");
        let code = http_response.status();
        let body_bytes = http_response.bytes()?;
        Ok(FullRegistryResponse {
            body_bytes,
            content_type: content_type_opt,
            docker_content_digest: docker_content_digest_opt,
            location_header,
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

    pub fn location_header(&self) -> Option<&String> {
        self.location_header.as_ref()
    }
}

pub trait RegistryResponse {
    fn success(&self) -> bool;

    fn content_type(&self) -> Option<&str>;

    fn status_code(&self) -> u16;

    fn string_body(self) -> String;
}

/// 一个简单的Registry的Response，只包含状态码
pub struct RawRegistryResponse {
    response: Response,
}

impl RegistryResponse for RawRegistryResponse {
    fn success(&self) -> bool {
        self.response.status().is_success()
    }

    fn content_type(&self) -> Option<&str> {
        let header_map = self.response.headers();
        header_map.get("content-type").map(|value| value.to_str()).and_then(|x| match x {
            Ok(str) => Some(str),
            Err(_) => None,
        })
    }

    fn status_code(&self) -> u16 {
        self.response.status().as_u16()
    }

    fn string_body(mut self) -> String {
        match self.response.content_length() {
            None => {
                let mut string = String::new();
                let _result = self.response.read_to_string(&mut string);
                string
            }
            Some(len) => {
                if len == 0 {
                    String::default()
                } else {
                    let mut string = String::with_capacity(len as usize);
                    let _result = self.response.read_to_string(&mut string);
                    string
                }
            }
        }
    }
}

pub struct ClientRequest<'a, B: Serialize + ?Sized> {
    path: &'a str,
    scope: Option<&'a str>,
    method: Method,
    accept: &'a [RegContentType],
    body: Option<&'a B>,
    request_content_type: Option<&'a RegContentType>,
    token_type: TokenType,
}

impl<'a, B: Serialize + ?Sized> ClientRequest<'a, B> {
    pub fn new(
        path: &'a str,
        scope: Option<&'a str>,
        method: Method,
        accept: &'a [RegContentType],
        body: Option<&'a B>,
        token_type: TokenType,
    ) -> ClientRequest<'a, B> {
        ClientRequest {
            path,
            scope,
            method,
            accept,
            body,
            request_content_type: None,
            token_type,
        }
    }

    pub fn new_head_request(path: &'a str, scope: Option<&'a str>, token_type: TokenType) -> ClientRequest<'a, B> {
        ClientRequest {
            path,
            scope,
            method: Method::HEAD,
            accept: &[],
            body: None,
            request_content_type: None,
            token_type,
        }
    }

    pub fn new_get_request(path: &'a str, scope: Option<&'a str>, accept: &'a [RegContentType]) -> ClientRequest<'a, B> {
        ClientRequest {
            path,
            scope,
            method: Method::GET,
            accept,
            body: None,
            request_content_type: None,
            token_type: TokenType::Pull,
        }
    }

    pub fn new_with_content_type(
        path: &'a str,
        scope: Option<&'a str>,
        method: Method,
        accept: &'a [RegContentType],
        body: Option<&'a B>,
        content_type: &'a RegContentType,
        token_type: TokenType,
    ) -> ClientRequest<'a, B> {
        ClientRequest {
            path,
            scope,
            method,
            accept,
            body,
            request_content_type: Some(content_type),
            token_type,
        }
    }
}
