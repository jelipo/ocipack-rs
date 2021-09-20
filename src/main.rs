mod registry_client;
mod reg;

use std::io::Read;
use crate::reg::registry::Registry;


fn main() -> anyhow::Result<()> {
    let registry = Registry::open("www.xxx.com".into_string())?;
    let info = registry.image_manager.get_image_info("ubuntu:20.04");


    Ok(())
}