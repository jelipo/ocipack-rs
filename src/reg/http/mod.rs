use std::str::FromStr;
use reqwest::blocking::{Client, Request, Response};
use reqwest::header::HeaderMap;
use reqwest::{Method, Url};
use serde::Serialize;
use anyhow::Result;

pub mod client;
pub mod download;
pub mod auth;

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

fn do_request_raw<T: Serialize + ?Sized>(
    client: &Client,
    url: &str,
    method: Method,
    http_auth_opt: &Option<HttpAuth>,
    body: Option<&T>,
) -> Result<Response> {
    let request = build_request(client, url, method, http_auth_opt, body)?;
    let http_response = client.execute(request)?;
    Ok(http_response)
}

fn build_request<T: Serialize + ?Sized>(
    client: &Client,
    url: &str,
    method: Method,
    http_auth_opt: &Option<HttpAuth>,
    body: Option<&T>,
) -> Result<Request> {
    let url = Url::from_str(url)?;
    let mut builder = client.request(method, url);
    if let Some(http_auth) = http_auth_opt {
        match http_auth {
            HttpAuth::BasicAuth { username, password } => {
                builder = builder.basic_auth(username, Some(password));
            }
            HttpAuth::BearerToken { token } => builder = builder.bearer_auth(token)
        }
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