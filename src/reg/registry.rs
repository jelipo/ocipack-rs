use std::path::Path;
use std::rc::Rc;

use anyhow::Result;

use crate::reg::client::RegistryHttpClient;
use crate::reg::home::HomeDir;
use crate::reg::image::ImageManager;

pub struct Registry {
    pub image_manager: ImageManager,
}

impl Registry {
    pub fn open(registry_addr: String) -> Result<Registry> {
        let client = RegistryHttpClient::new(registry_addr.clone(), "jelipo", "")?;
        let client_rc = Rc::new(client);
        let home_dir = HomeDir::new_home_dir(Path::new("C:/Users/cao/Desktop/caches"));
        let home_dir_rc = Rc::new(home_dir);
        let image = ImageManager::new(registry_addr.clone(), client_rc.clone(), home_dir_rc);
        Ok(Registry {
            image_manager: image,
        })
    }
}
