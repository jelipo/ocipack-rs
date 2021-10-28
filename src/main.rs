#![feature(exclusive_range_pattern)]

use anyhow::Result;

use crate::reg::registry::Registry;
use crate::reg::Reference;

mod reg;
mod registry_client;
mod util;

fn main() -> Result<()> {
    let registry = Registry::open("https://harbor.jelipo.com".to_string())?;
    let reference = Reference {
        image_name: "private/mongo",
        reference: "5.0.2",
    };
    let mainfest = registry.image_manager.manifests(&reference)?;
    let vec = mainfest.layers;
    let layer = &vec[0];
    let disgest = &layer.digest;
    registry.image_manager.blobs_download(&reference.image_name, disgest)?;
    Ok(())
}
