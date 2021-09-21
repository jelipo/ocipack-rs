use std::borrow::Borrow;
use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

use anyhow::Result;

use crate::reg::client::HttpClient;
use crate::reg::Reference;

pub struct ImageManager {
    registry_addr: String,
    http_client: Rc<HttpClient>,
}

impl ImageManager {
    pub fn new(registry_addr: String, client: Rc<HttpClient>) -> ImageManager {
        ImageManager {
            registry_addr,
            http_client: client,
        }
    }

    pub fn get_image_info(&self, name: &str) -> Result<String> {
        return self.http_client.request();
    }
}