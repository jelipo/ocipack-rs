use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use anyhow::Result;

use crate::reg::docker::http::client::RegistryHttpClient;
use crate::reg::docker::http::RegistryAuth;
use crate::reg::docker::ImageManager;
use crate::reg::home::HomeDir;

pub struct Registry {
    pub image_manager: ImageManager,
}

impl Registry {
    pub fn open(
        registry_addr: String,
        auth: Option<RegistryAuth>,
        home_dir: Rc<HomeDir>,
    ) -> Result<Registry> {
        let client = RegistryHttpClient::new(registry_addr.clone(), auth)?;
        let client_rc = Rc::new(RefCell::new(client));
        let image = ImageManager::new(registry_addr.clone(), client_rc.clone(), home_dir);
        Ok(Registry {
            image_manager: image,
        })
    }
}
