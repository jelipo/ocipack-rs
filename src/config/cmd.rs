use std::str::FromStr;

use anyhow::Error;
use clap::{ArgEnum, Parser, Subcommand};

#[derive(Parser)]
#[clap(about, version, author = "jelipo")]
pub enum CmdArgs {
    Build(BuildArgs)
}

#[derive(clap::Args)]
#[clap(about, author, version)]
pub struct BuildArgs {
    #[clap(long, short)]
    source: SourceType,
}

impl FromStr for SourceType {
    type Err = anyhow::Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        let potion = arg.chars().position(|c| c == ':')
            .ok_or(Error::msg("error source"))?;
        let source_type = &arg[..potion];
        Ok(match source_type {
            "dockerfile" => SourceType::Dockerfile(arg[potion + 1..].to_string()),
            _ => return Err(Error::msg(format!("unknown source type: {}", source_type)))
        })
    }
}

pub enum SourceType {
    Dockerfile(String)
}

#[derive(clap::Args)]
#[clap(about, author, version)]
pub struct DockerfileArgs {
    path: String,
}

#[derive(Subcommand)]
#[clap(about, version, author)]
pub enum ToType {
    Registry(RegistryArgs)
}

#[derive(clap::Args)]
#[clap(about, author, version)]
pub struct RegistryArgs {
    tag: String,
}


