use std::io::Read;
use std::str::FromStr;

use anyhow::Result;
use reqwest::{Method, Url};
use reqwest::blocking::{Body, Client, Request, Response};
use reqwest::header::HeaderMap;
use serde::Serialize;

use crate::reg::RegContentType;

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

pub enum RequestBody<'a, T: Serialize + ?Sized> {
    JSON(&'a T),
    Read(Body),
}

fn do_request_raw<T: Serialize + ?Sized>(
    client: &Client, url: &str, method: Method, http_auth_opt: Option<&HttpAuth>,
    accepts: &[RegContentType], body: Option<&T>, content_type: Option<&RegContentType>,
) -> Result<Response> {
    let request_body = body.map(|json| RequestBody::JSON(json));
    let request = build_request::<T>(client, url, method, http_auth_opt, accepts, request_body, content_type)?;
    let http_response = client.execute(request)?;
    Ok(http_response)
}

fn do_request_raw_read<R: Read + Send + 'static>(
    client: &Client, url: &str, method: Method, http_auth_opt: Option<&HttpAuth>,
    _accept: Option<&RegContentType>, body: Option<R>, size: u64,
) -> Result<Response> {
    let request_body = body.map(|read| RequestBody::Read(Body::sized(read, size)));
    let request = build_request::<String>(client, url, method, http_auth_opt, &[], request_body, None)?;
    let http_response = client.execute(request)?;
    Ok(http_response)
}

fn build_request<T: Serialize + ?Sized>(
    client: &Client, url: &str, method: Method, http_auth_opt: Option<&HttpAuth>,
    accepts: &[RegContentType], body: Option<RequestBody<T>>, content_type: Option<&RegContentType>,
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
    // Set accept header
    let mut accepts_str = Vec::with_capacity(accepts.len());
    for accept in accepts {
        accepts_str.push(accept.val())
    }
    if accepts.len() > 0 {
        builder = builder.header("Accept", accepts_str.join(";"));
    }
    if let Some(content_type) = content_type {
        builder = builder.header("Content-Type", content_type.val());
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
