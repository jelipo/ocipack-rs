mod registry_client;
mod reg;

use std::io::Read;
use crate::reg::registry::Registry;
use anyhow::Result;


fn main() -> Result<()> {
    let registry = Registry::open("www.xxx.com".to_string())?;
    let info = registry.image_manager.get_image_info("ubuntu:20.04");
    println!("{}", info?);

    Ok(())
}