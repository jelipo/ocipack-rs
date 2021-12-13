use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use crate::reg::home::HomeDir;
use crate::reg::http::client::RegistryHttpClient;
use crate::reg::http::RegistryAuth;
use crate::reg::oci::ImageManager;

pub struct OciRegistry {
    pub image_manager: ImageManager,
}

impl OciRegistry {
    pub fn open(
        registry_addr: String,
        auth: Option<RegistryAuth>,
        home_dir: Rc<HomeDir>,
    ) -> Result<OciRegistry> {
        let client = RegistryHttpClient::new(registry_addr.clone(), auth)?;
        let client_rc = Rc::new(RefCell::new(client));
        let image = ImageManager::new(registry_addr.clone(), client_rc.clone(), home_dir);
        Ok(OciRegistry {
            image_manager: image,
        })
    }
}
