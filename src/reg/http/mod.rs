use std::io::Read;
use std::str::FromStr;

use anyhow::Result;
use reqwest::{Method, Url};
use reqwest::blocking::{Body, Client, Request, Response};
use reqwest::header::HeaderMap;
use serde::Serialize;

pub mod auth;
pub mod client;
pub mod download;
pub mod upload;

#[derive(Clone)]
pub struct RegistryAuth {
    pub username: String,
    pub password: String,
}

#[derive(Clone)]
pub enum HttpAuth {
    BasicAuth { username: String, password: String },
    BearerToken { token: String },
}

pub struct RegistryContentType(&'static str);

pub enum RequestBody<'a, T: Serialize + ?Sized> {
    JSON(&'a T),
    Read(Body),
}

impl RegistryContentType {
    /// Docker content-type
    pub const DOCKER_MANIFEST: Self = Self("application/vnd.docker.distribution.manifest.v2+json");
    pub const DOCKER_MANIFEST_LIST: Self = Self("application/vnd.docker.distribution.manifest.list.v2+json");

    /// OCI content-type
    pub const OCI_MANIFEST: Self = Self("application/vnd.oci.image.manifest.v1+json");
    pub const ALL: Self = Self(" */*");

    fn get_value(&self) -> &'static str {
        self.0
    }
}

fn do_request_raw<T: Serialize + ?Sized>(
    client: &Client, url: &str, method: Method, http_auth_opt: Option<&HttpAuth>,
    accept: Option<&RegistryContentType>, body: Option<&T>, content_type: Option<&RegistryContentType>,
) -> Result<Response> {
    let request_body = body.map(|json| RequestBody::JSON(json));
    let request = build_request::<T>(client, url, method, http_auth_opt, accept, request_body, content_type)?;
    let http_response = client.execute(request)?;
    Ok(http_response)
}

fn do_request_raw_read<R: Read + Send + 'static>(
    client: &Client, url: &str, method: Method, http_auth_opt: Option<&HttpAuth>,
    accept: Option<&RegistryContentType>, body: Option<R>, size: u64,
) -> Result<Response> {
    let request_body = body.map(|read| RequestBody::Read(Body::sized(read, size)));
    let request = build_request::<String>(client, url, method, http_auth_opt, accept, request_body, None)?;
    let http_response = client.execute(request)?;
    Ok(http_response)
}

fn build_request<T: Serialize + ?Sized>(
    client: &Client, url: &str, method: Method, http_auth_opt: Option<&HttpAuth>,
    accept: Option<&RegistryContentType>, body: Option<RequestBody<T>>, content_type: Option<&RegistryContentType>,
) -> Result<Request> {
    let url = Url::from_str(url)?;
    let mut builder = client.request(method, url);
    match http_auth_opt {
        None => {}
        Some(HttpAuth::BasicAuth { username, password }) => {
            builder = builder.basic_auth(username, Some(password));
        }
        Some(HttpAuth::BearerToken { token }) => builder = builder.bearer_auth(token)
    }
    if let Some(reg_accept) = accept {
        builder = builder.header("Accept", reg_accept.get_value());
    }
    if let Some(content_type) = content_type {
        builder = builder.header("Content-Type", content_type.get_value());
    }
    match body {
        None => {}
        Some(RequestBody::JSON(json_body)) => {
            let json_str = serde_json::to_string(json_body)?;
            builder = builder.body(json_str)
        }
        Some(RequestBody::Read(read)) => builder = builder.body(read)
    }
    Ok(builder.build()?)
}

fn get_header(headers: &HeaderMap, header_name: &str) -> Option<String> {
    headers.get(header_name).and_then(|value| match value.to_str() {
        Ok(str) => Some(String::from(str)),
        Err(_) => None,
    })
}
