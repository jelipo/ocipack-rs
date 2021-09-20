use std::cell::RefCell;
use std::rc::Rc;

use crate::reg::client::HttpClient;
use crate::reg::Reference;

pub struct ImageManager {
    registry_addr: String,
    http_client: HttpClient,
}

impl ImageManager {
    pub fn new(registry_addr: String, client: HttpClient) -> ImageManager {
        ImageManager {
            registry_addr,
            http_client: client,
        }
    }

    pub fn get_image_info(&self, name: &str) -> String {
        self.http_client.http_do_somthing(name)
        // TODO
    }
}