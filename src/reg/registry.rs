use std::rc::Rc;

use anyhow::Result;

use crate::reg::client::RegistryHttpClient;
use crate::reg::image::ImageManager;

pub struct Registry {
    pub image_manager: ImageManager,
}

impl Registry {
    pub fn open(registry_addr: String) -> Result<Registry> {
        let client = RegistryHttpClient::new(registry_addr.clone(), "jelipo", "")?;
        let client_rc = Rc::new(client);
        let image = ImageManager::new(registry_addr.clone(), client_rc.clone());

        Ok(Registry {
            image_manager: image,
        })
    }
}
