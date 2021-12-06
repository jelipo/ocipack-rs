use std::collections::HashMap;
use std::option::Option::Some;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Error, Result};
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::Method;
use serde::Deserialize;

use crate::reg::docker::http::{do_request_raw, get_header, HttpAuth};

pub struct RegTokenHandler {
    registry_addr: String,
    basic_auth: Option<HttpAuth>,
    scope_token_map: HashMap<String, InnerToken>,
    client: Client,
    authenticate_adapter: Option<AuthenticateAdapter>,
}

impl RegTokenHandler {
    pub fn new_reg_token_handler(
        registry_addr: String, basic_auth: Option<HttpAuth>, client: Client,
    ) -> RegTokenHandler {
        RegTokenHandler {
            registry_addr,
            basic_auth,
            scope_token_map: HashMap::new(),
            client,
            authenticate_adapter: None,
        }
    }

    pub fn token(&mut self, scope_opt: Option<&str>) -> Result<String> {
        let scope = if let Some(scope) = scope_opt {
            scope
        } else {
            ""
        };
        if let Some(token) = self.token_from_cache(scope)? {
            Ok(token)
        } else {
            let token = self.update_token_to_cache(scope_opt)?;
            Ok(token)
        }
    }

    /// 从内存缓存中拿取未过期的Token
    fn token_from_cache(&self, scope: &str) -> Result<Option<String>> {
        if let Some(inner_token) = self.scope_token_map.get(scope) {
            let second_time_now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
            if second_time_now < inner_token.expire_second_time {
                return Ok(Some(inner_token.token.clone()));
            }
        }
        return Ok(None);
    }

    fn update_token_to_cache(&mut self, scope_opt: Option<&str>) -> Result<String> {
        let adapter = match &self.authenticate_adapter {
            None => {
                let new_adapter = AuthenticateAdapter::new_authenticate_adapter(
                    &self.registry_addr,
                    &self.client,
                )?;
                self.authenticate_adapter = Some(new_adapter);
                self.authenticate_adapter.as_ref().unwrap()
            }
            Some(adapter) => adapter,
        };
        let token_response = adapter.new_token(scope_opt, self.basic_auth.as_ref(), &self.client)?;
        let scope = if let Some(scope) = scope_opt {
            scope
        } else {
            ""
        };
        let second_time_now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        self.scope_token_map.insert(
            scope.to_string(),
            InnerToken {
                token: token_response.token.clone(),
                expire_second_time: second_time_now + token_response.expires_in as u64,
            },
        );
        Ok(token_response.token)
    }

    fn build_adapter(&self, _scope: &Option<String>) {}
}

pub struct AuthenticateAdapter {
    realm: String,
    service: String,
}

impl AuthenticateAdapter {
    pub fn new_authenticate_adapter(
        registry_addr: &str,
        client: &Client,
    ) -> Result<AuthenticateAdapter> {
        let bearer_url = format!("{}/v2/", registry_addr);
        let http_response =
            do_request_raw::<u8>(client, bearer_url.as_str(), Method::GET, None, &None, None)?;
        let www_authenticate = get_header(http_response.headers(), "Www-Authenticate")
            .expect("Www-Authenticate header not found");
        let regex = Regex::new("^Bearer realm=\"(?P<realm>.*)\",service=\"(?P<service>.*)\".*")?;
        let captures = regex.captures(www_authenticate.as_str()).expect(&format!(
            "www_authenticate header not support:{}",
            www_authenticate.as_str()
        ));
        let realm = &captures["realm"];
        let service = &captures["service"];
        Ok(AuthenticateAdapter {
            realm: realm.to_string(),
            service: service.to_string(),
        })
    }

    pub fn new_token(
        &self, scope: Option<&str>, basic_auth: Option<&HttpAuth>, client: &Client,
    ) -> Result<TokenResponse> {
        let mut url = format!("{}?service={}", &self.realm, &self.service);
        if let Some(scope_raw) = scope {
            url = url + "&scope=repository:" + scope_raw + ":pull";
        }
        let http_response =
            do_request_raw::<u8>(client, url.as_str(), Method::GET, basic_auth, &None, None)?;
        let status = http_response.status();
        if !status.is_success() {
            return Err(Error::msg(format!("get token failed,code:{}", status.as_str())));
        }
        Ok(http_response.json::<TokenResponse>()?)
    }
}

#[derive(Deserialize)]
pub struct TokenResponse {
    token: String,
    expires_in: usize,
}

struct InnerToken {
    token: String,
    expire_second_time: u64,
}
