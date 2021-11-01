use std::char::UNICODE_VERSION;
use std::collections::HashMap;
use std::option::Option::Some;
use std::str::FromStr;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Error, Result};
use regex::Regex;
use reqwest::{Method, Url};
use reqwest::blocking::{Client, Response};
use serde::Deserialize;

use crate::reg::http::{do_request_raw, get_header, HttpAuth, RegistryAuth};

pub struct RegTokenHandler {
    registry_addr: String,
    basic_auth: Option<HttpAuth>,
    scope_token_map: HashMap<String, InnerToken>,
    token: Option<String>,
    expire_second_time: u64,
    client: Client,
    authenticate_adapter: Option<AuthenticateAdapter>,
}

impl RegTokenHandler {
    pub fn new_reg_token_handler(registry_addr: String, basic_auth: Option<HttpAuth>, client: Client) -> RegTokenHandler {
        RegTokenHandler {
            registry_addr,
            basic_auth,
            scope_token_map: HashMap::new(),
            token: None,
            expire_second_time: 0,
            client,
            authenticate_adapter: None,
        }
    }

    pub fn token(&mut self, scope: Option<String>) -> Result<String> {
        match &self.token {
            None => self.update_token(&scope)?,
            Some(token) => {
                if self.is_token_expire_now()? {
                    self.update_token(&scope)?
                } else { return Ok(token.clone()); }
            }
        }
        self.token.clone().ok_or(Error::msg("get token failed"))
    }

    fn request() {}

    /// 存储的Token是否过期了
    fn is_token_expire_now(&self) -> Result<bool> {
        let second_time_now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        Ok(second_time_now > self.expire_second_time)
    }

    fn update_token(&mut self, scope: &Option<String>) -> Result<()> {
        Ok(())
    }

    fn build_adapter(&self, scope: &Option<String>) {}
}

pub struct AuthenticateAdapter {
    realm: String,
    service: String,
}

impl AuthenticateAdapter {
    pub fn new_authenticate_adapter(registry_addr: &str, client: &Client) -> Result<AuthenticateAdapter> {
        let bearer_url = format!("{}/v2", registry_addr);
        let http_response = do_request_raw::<u8>(
            client, bearer_url.as_str(), Method::GET, &None, None)?;
        let www_authenticate = get_header(http_response.headers(), "Www-Authenticate")
            .expect("Www-Authenticate header not found");
        let regex = Regex::new("^Bearer realm=\"(?P<realm>.*)\",service=\"(?P<service>.*)\".*")?;
        let captures = regex.captures(www_authenticate.as_str())
            .expect(&format!("www_authenticate header not support:{}", www_authenticate.as_str()));
        let realm = &captures["realm"];
        let service = &captures["service"];
        Ok(AuthenticateAdapter {
            realm: realm.to_string(),
            service: service.to_string(),
        })
    }

    pub fn new_token(&self, scope: Option<String>, basic_auth: &Option<HttpAuth>, client: &Client) -> Result<TokenResponse> {
        let mut url = format!("{}?service={}", &self.realm, &self.service);
        if let Some(scope_raw) = scope {
            url = url + "&scope=" + scope_raw.as_str();
        }
        let http_response = do_request_raw::<u8>(
            client, url.as_str(), Method::GET, basic_auth, None)?;
        let status = http_response.status();
        if !status.is_success() {
            return Err(Error::msg(format!("get token failed,code:{}", status.as_str())));
        }
        Ok(http_response.json::<TokenResponse>()?)
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    token: String,
    expires_in: usize,
}

struct InnerToken {
    token: String,
    expire_second_time: u64,
}

