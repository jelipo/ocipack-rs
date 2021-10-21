mod reg;
mod registry_client;

use crate::reg::registry::Registry;
use crate::reg::Reference;
use anyhow::Result;

fn main() -> Result<()> {
    let registry = Registry::open("https://harbor.jelipo.com".to_string())?;
    let reference = Reference {
        image_name: "oci-test/hello-worlda".to_string(),
        reference: "1.0".to_string(),
    };
    let info = registry.image_manager.get_manifests(&reference)?;
    println!("{:?}", info);

    Ok(())
}
