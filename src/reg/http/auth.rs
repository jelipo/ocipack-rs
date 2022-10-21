use std::collections::HashMap;
use std::option::Option::Some;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use chrono::Local;
use fantasy_util::time::system_time::SystemLocalTime;
use log::warn;
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::Method;
use serde::Deserialize;

use crate::reg::http::{do_request_raw, get_header, HttpAuth};

pub struct RegTokenHandler {
    registry_addr: String,
    basic_auth: Option<HttpAuth>,
    token_cache: TokenCache,
    client: Client,
    authenticate_adapter: Option<AuthenticateAdapter>,
}

impl RegTokenHandler {
    pub fn new_reg_token_handler(registry_addr: String, basic_auth: Option<HttpAuth>, client: Client) -> RegTokenHandler {
        RegTokenHandler {
            registry_addr,
            basic_auth,
            client,
            authenticate_adapter: None,
            token_cache: TokenCache::default(),
        }
    }

    pub fn token(&mut self, scope_opt: Option<&str>, token_type: TokenType) -> Result<String> {
        let scope = match scope_opt {
            None => "",
            Some(scope) => scope,
        };
        match self.token_cache.get_token(scope, token_type.clone()) {
            None => {
                let (token, expire_second_time) = self.get_remote_token(scope_opt, token_type.clone())?;
                self.token_cache.put_token(scope, token_type, expire_second_time, &token);
                Ok(token)
            }
            Some(token) => Ok(token),
        }
    }

    fn get_remote_token(&mut self, scope_opt: Option<&str>, token_type: TokenType) -> Result<(String, u64)> {
        let adapter = match &self.authenticate_adapter {
            None => {
                let new_adapter = AuthenticateAdapter::new_authenticate_adapter(&self.registry_addr, &self.client)
                    .map_err(|err| anyhow!("get token failed: {}", err))?;
                self.authenticate_adapter = Some(new_adapter);
                self.authenticate_adapter.as_ref().unwrap()
            }
            Some(adapter) => adapter,
        };
        let token_response = adapter.new_token(scope_opt, self.basic_auth.as_ref(), &self.client, token_type)?;
        let expires_in = token_response.expires_in.unwrap_or_else(|| {
            warn!("'expires_in' not found in token response. Set the expires time to 60 second by default.");
            60
        });
        Ok((token_response.token, SystemLocalTime::unix_secs() + expires_in as u64))
    }
}

pub struct AuthenticateAdapter {
    realm: String,
    service: String,
}

impl AuthenticateAdapter {
    pub fn new_authenticate_adapter(registry_addr: &str, client: &Client) -> Result<AuthenticateAdapter> {
        let bearer_url = format!("{}/v2/", registry_addr);
        let http_response = do_request_raw::<u8>(client, bearer_url.as_str(), Method::GET, None, &[], None, None)?;
        let www_authenticate = get_header(http_response.headers(), "Www-Authenticate")
            .ok_or_else(|| anyhow!("'Www-Authenticate' header not found"))?;
        let regex = Regex::new("^Bearer realm=\"(?P<realm>.*)\",service=\"(?P<service>.*)\".*")?;
        let captures = regex
            .captures(www_authenticate.as_str())
            .ok_or_else(|| anyhow!("www_authenticate header not support:{}", www_authenticate.as_str()))?;
        let realm = &captures["realm"];
        let service = &captures["service"];
        Ok(AuthenticateAdapter {
            realm: realm.to_string(),
            service: service.to_string(),
        })
    }

    pub fn new_token(
        &self,
        scope: Option<&str>,
        basic_auth: Option<&HttpAuth>,
        client: &Client,
        token_type: TokenType,
    ) -> Result<TokenResponse> {
        let mut url = format!("{}?service={}", &self.realm, &self.service);
        if let Some(scope_raw) = scope {
            match token_type {
                TokenType::PushAndPull => url = url + "&scope=repository:" + scope_raw + ":pull,push",
                TokenType::Pull => url = url + "&scope=repository:" + scope_raw + ":pull",
            }
        }
        let http_response = do_request_raw::<u8>(client, url.as_str(), Method::GET, basic_auth, &[], None, None)?;
        let status = http_response.status();
        let response_text = http_response.text().unwrap_or_else(|_| String::new());
        if !status.is_success() {
            return Err(anyhow!(
                "get token failed,code: {}. response: {}",
                status.as_str(),
                &response_text
            ));
        }
        serde_json::from_str::<TokenResponse>(&response_text).map_err(|err| {
            anyhow!(
                "deserialization 'get token' response failed: {}. response: {}.",
                err,
                response_text
            )
        })
    }
}

#[derive(Deserialize)]
pub struct TokenResponse {
    token: String,
    expires_in: Option<usize>,
}

struct InnerToken {
    token: String,
    expire_second_time: u64,
}

#[derive(Clone)]
pub enum TokenType {
    PushAndPull,
    Pull,
}

#[derive(Default)]
struct TokenCache {
    push_and_pull_map: HashMap<String, InnerToken>,
    pull_map: HashMap<String, InnerToken>,
}

impl TokenCache {
    pub fn get_token(&mut self, scope: &str, token_type: TokenType) -> Option<String> {
        match self.get_token_with_type(scope, TokenType::PushAndPull) {
            None => match token_type {
                TokenType::PushAndPull => None,
                TokenType::Pull => self.get_token_with_type(scope, TokenType::Pull),
            },
            Some(token) => Some(token),
        }
    }

    fn get_token_with_type(&mut self, scope: &str, token_type: TokenType) -> Option<String> {
        match token_type {
            TokenType::PushAndPull => get_token(scope, &mut self.push_and_pull_map),
            TokenType::Pull => get_token(scope, &mut self.pull_map),
        }
    }

    fn put_token(&mut self, scope: &str, token_type: TokenType, expire_second_time: u64, token: &str) {
        let inner_token = InnerToken {
            token: token.to_string(),
            expire_second_time,
        };
        match token_type {
            TokenType::PushAndPull => self.push_and_pull_map.insert(scope.to_string(), inner_token),
            TokenType::Pull => self.pull_map.insert(scope.to_string(), inner_token),
        };
    }
}

fn get_token(scope: &str, map: &mut HashMap<String, InnerToken>) -> Option<String> {
    if let Some(inner_token) = map.get(scope) {
        let second_time_now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        if second_time_now < inner_token.expire_second_time {
            return Some(inner_token.token.clone());
        } else {
            map.remove(scope);
        }
    }
    None
}
