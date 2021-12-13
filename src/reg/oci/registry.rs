use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use crate::reg::home::HomeDir;
use crate::reg::http::client::RegistryHttpClient;
use crate::reg::http::RegistryAuth;
use crate::reg::oci::OciImageManager;

pub struct OciRegistry {
    pub oci_image_manager: OciImageManager,
}

impl OciRegistry {
    pub fn open(
        registry_addr: String,
        auth: Option<RegistryAuth>,
        home_dir: Rc<HomeDir>,
    ) -> Result<OciRegistry> {
        let client = RegistryHttpClient::new(registry_addr.clone(), auth)?;
        let client_rc = Rc::new(RefCell::new(client));
        let image = OciImageManager::new(registry_addr.clone(), client_rc.clone(), home_dir);
        Ok(OciRegistry {
            oci_image_manager: image,
        })
    }
}
