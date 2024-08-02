use std::str::FromStr;

use anyhow::Result;
use reqwest::{Body, Client, Request, Response};
use reqwest::{Method, Url};
use reqwest::header::{CONTENT_LENGTH, HeaderMap, HeaderValue};
use serde::Serialize;
use tokio::io::AsyncRead;

use crate::container::RegContentType;

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
    Json(&'a T),
    Read(Body),
}

async fn do_request_raw<T: Serialize + ?Sized>(
    client: &Client,
    url: &str,
    method: Method,
    http_auth_opt: Option<&HttpAuth>,
    accepts: &[RegContentType],
    body: Option<&T>,
    content_type: Option<&RegContentType>,
) -> Result<Response> {
    let request_body = body.map(|json| RequestBody::Json(json));
    let request = build_request::<T>(client, url, method, http_auth_opt, accepts, request_body, content_type)?;
    let http_response = client.execute(request).await?;
    Ok(http_response)
}

async fn do_request_raw_read<R: AsyncRead + Send>(
    client: &Client,
    url: &str,
    method: Method,
    http_auth_opt: Option<&HttpAuth>,
    accepts: &[RegContentType],
    body: Option<R>,
    size: u64,
) -> Result<Response> {
    let request_body = body.map(|read| RequestBody::Read(Body::wrap_stream(read)));

    let mut request = build_request::<String>(client, url, method, http_auth_opt, accepts, request_body, None)?;
    request.headers_mut().insert(CONTENT_LENGTH, HeaderValue::from(size));
    let http_response = client.execute(request).await?;
    Ok(http_response)
}

fn build_request<T: Serialize + ?Sized>(
    client: &Client,
    url: &str,
    method: Method,
    http_auth_opt: Option<&HttpAuth>,
    accepts: &[RegContentType],
    body: Option<RequestBody<T>>,
    content_type: Option<&RegContentType>,
) -> Result<Request> {
    let url = Url::from_str(url)?;
    let mut builder = client.request(method, url);
    match http_auth_opt {
        None => {}
        Some(HttpAuth::BasicAuth { username, password }) => builder = builder.basic_auth(username, Some(password)),
        Some(HttpAuth::BearerToken { token }) => builder = builder.bearer_auth(token),
    }
    // Set accept header
    let mut accepts_str = Vec::with_capacity(accepts.len());
    for accept in accepts {
        accepts_str.push(accept.val())
    }
    if !accepts.is_empty() {
        builder = builder.header("Accept", accepts_str.join(","));
    }
    if let Some(content_type) = content_type {
        builder = builder.header("Content-Type", content_type.val());
    }
    match body {
        None => {}
        Some(RequestBody::Json(json_body)) => {
            let json_str = serde_json::to_string(json_body)?;
            builder = builder.body(json_str)
        }
        Some(RequestBody::Read(body)) => builder = builder.body(body),
    }
    Ok(builder.build()?)
}

fn get_header(headers: &HeaderMap, header_name: &str) -> Option<String> {
    headers.get(header_name).and_then(|value| match value.to_str() {
        Ok(str) => Some(String::from(str)),
        Err(_) => None,
    })
}
