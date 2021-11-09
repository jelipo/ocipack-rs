use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use reqwest::{Method, Url};
use reqwest::blocking::{Client, Request, Response};
use reqwest::header::HeaderMap;
use serde::Serialize;

pub mod auth;
pub mod client;
pub mod download;

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

pub struct RegistryAccept(&'static str);

impl RegistryAccept {
    pub const APPLICATION_VND_DOCKER_DISTRIBUTION_MANIFEST_V2JSON: Self = Self("application/vnd.docker.distribution.manifest.v2+json");
    pub const ALL: Self = Self("*/*");

    fn get_value(&self) -> &'static str {
        self.0
    }
}

fn do_request_raw<T: Serialize + ?Sized>(
    client: &Client,
    url: &str,
    method: Method,
    http_auth_opt: &Option<HttpAuth>,
    accept: &Option<RegistryAccept>,
    body: Option<&T>,
) -> Result<Response> {
    let request = build_request(client, url, method, http_auth_opt, accept, body)?;
    let http_response = client.execute(request)?;
    Ok(http_response)
}

fn build_request<T: Serialize + ?Sized>(
    client: &Client,
    url: &str,
    method: Method,
    http_auth_opt: &Option<HttpAuth>,
    accept: &Option<RegistryAccept>,
    body: Option<&T>,
) -> Result<Request> {
    let url = Url::from_str(url)?;
    let mut builder = client.request(method, url);
    if let Some(http_auth) = http_auth_opt {
        match http_auth {
            HttpAuth::BasicAuth { username, password } => {
                builder = builder.basic_auth(username, Some(password));
            }
            HttpAuth::BearerToken { token } => builder = builder.bearer_auth(token),
        }
    }
    if let Some(reg_accept) = accept {
        builder = builder.header("Accept", reg_accept.get_value());
    }
    if let Some(body_o) = body {
        builder = builder.json::<T>(body_o)
    }
    Ok(builder.build()?)
}

fn get_header(headers: &HeaderMap, header_name: &str) -> Option<String> {
    headers.get(header_name).and_then(|value| match value.to_str() {
        Ok(str) => Some(String::from(str)),
        Err(_) => None,
    })
}
