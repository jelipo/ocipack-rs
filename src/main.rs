#![feature(exclusive_range_pattern)]

use crate::reg::http::RegistryAuth;
use anyhow::Result;
use std::thread::sleep;
use std::time::Duration;

use crate::reg::image::ManifestLayer;

use crate::reg::registry::Registry;
use crate::reg::Reference;

mod reg;
mod registry_client;
mod util;

fn main() -> Result<()> {
    let auth = RegistryAuth {
        username: "jelipo".to_string(),
        password: "".to_string(),
    };
    let registry = Registry::open("https://harbor.jelipo.com".to_string(), Some(auth))?;
    let reference = Reference {
        image_name: "private/mongo",
        reference: "5.0.2",
    };
    let manifest = registry.image_manager.manifests(&reference)?;
    let vec = manifest.layers;
    let layer = &vec[0];
    download(layer, &registry, &reference)?;
    sleep(Duration::from_secs(60 * 5));
    Ok(())
}

fn download(
    manifest_layer: &ManifestLayer,
    registry: &Registry,
    reference: &Reference,
) -> Result<()> {
    let disgest = &manifest_layer.digest;
    let mut downloader = registry
        .image_manager
        .blobs_download(&reference.image_name, disgest)?;
    let _handle = downloader.start()?;
    let arc = downloader.download_temp();
    loop {
        sleep(Duration::from_secs(1));
        let temp = arc.lock().unwrap();
        println!("{}", temp.curr_size);
    }
    println!("{} 开始下载", manifest_layer.digest);
    Ok(())
}
