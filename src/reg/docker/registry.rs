use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Result;

use crate::reg::docker::DockerImageManager;
use crate::reg::home::HomeDir;
use crate::reg::http::client::RegistryHttpClient;
use crate::reg::http::RegistryAuth;

pub struct DockerRegistry {
    pub docker_image_manager: DockerImageManager,
}

impl DockerRegistry {
    pub fn open(
        registry_addr: String,
        auth: Option<RegistryAuth>,
        home_dir: Rc<HomeDir>,
    ) -> Result<DockerRegistry> {
        let client = RegistryHttpClient::new(registry_addr.clone(), auth)?;
        let client_rc = Rc::new(RefCell::new(client));
        let image = DockerImageManager::new(registry_addr.clone(), client_rc.clone(), home_dir);
        Ok(DockerRegistry {
            docker_image_manager: image,
        })
    }
}
