#![feature(exclusive_range_pattern)]

mod reg;
mod registry_client;
mod util;

use crate::reg::registry::Registry;
use crate::reg::Reference;
use anyhow::Result;

fn main() -> Result<()> {
    let registry = Registry::open("https://harbor.jelipo.com".to_string())?;
    let reference = Reference {
        image_name: "oci-test111/hello-world",
        reference: "1.0",
    };
    let info = registry.image_manager.exited(&reference)?;
    println!("{:?}", info);

    Ok(())
}
