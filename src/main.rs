mod registry_client;
mod reg;

use std::io::Read;
use crate::reg::registry::Registry;
use anyhow::Result;
use crate::reg::Reference;


fn main() -> Result<()> {
    let registry = Registry::open("https://harbor.jelipo.com".to_string())?;
    let reference = Reference {
        image_name: "oci-test/hello-world".to_string(),
        reference: "1.0".to_string()
    };
    let info = registry.image_manager.get_manifests(&reference)?;
    println!("{}", info);

    Ok(())
}