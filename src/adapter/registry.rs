use anyhow::{Error, Result};
use dockerfile_parser::{Dockerfile, FromInstruction, Instruction};

use crate::adapter::{ImageInfo, TargetImageAdapter, TargetInfo};
use crate::util::file::remove;

pub struct RegistryTargetAdapter {
    info: TargetInfo,
}

impl TargetImageAdapter for RegistryTargetAdapter {
    fn info(&self) -> &TargetInfo {
        &self.info
    }
}

impl RegistryTargetAdapter {
    pub fn new(image: &str) -> Result<RegistryTargetAdapter> {
        let temp_from = format!("FROM {}", image);
        let instruction = Dockerfile::parse(&temp_from)?.instructions.remove(0);
        let image_info = match instruction {
            Instruction::From(from) => ImageInfo {
                image_host: from.image_parsed.registry,
                image_name: from.image_parsed.image,
                reference: from.image_parsed.tag.or(from.image_parsed.hash)
                    .ok_or(Error::msg("can not found hash or tag"))?,
            },
            _ => return Err(Error::msg("")),
        };
        Ok(RegistryTargetAdapter {
            info: TargetInfo {
                image_info
            }
        })
    }
}

#[test]
fn test() -> Result<()> {
    RegistryTargetAdapter::new("harbor.jelipo.com/dsad/dwsdwsd:e231q")?;
    Ok(())
}


