use std::str::FromStr;

use anyhow::{anyhow, Error};
use clap::{ArgEnum, Parser, Subcommand};

#[derive(Parser)]
#[clap(about = "An image tool", version, author = "jelipo <me@jelipo.com>")]
pub enum CmdArgs {
    Build(BuildArgs),
    Transform,
}

#[derive(clap::Args)]
#[clap(about, author, version)]
pub struct BuildArgs {
    /// Allow insecure registry
    #[clap(long, short, parse(from_flag))]
    pub allow_insecure: bool,
    /// Source type
    /// Support 'dockerfile','cmd' type
    /// Example:'dockerfile:/path/to/.Dockerfile','cmd:my.reg.com/source/image:1.0'
    #[clap(long, short)]
    pub source: SourceType,
    /// [Option] Auth of pull source image. Example:'myname:mypass','myname:&{MY_PASSWORD_ENV}'
    #[clap(long)]
    pub source_auth: Option<BaseAuth>,

    /// Target type.
    /// Support 'registry'
    /// Example:'registry:my.reg.com/target/image:1.1'
    #[clap(long, short)]
    pub target: TargetType,
    /// [Option] Auth of push target image. Example:'myname:mypass','myname:&{MY_PASSWORD_ENV}'
    #[clap(long)]
    pub target_auth: Option<BaseAuth>,
    #[clap(long, arg_enum)]
    pub target_format: Option<TargetFormat>,
}

#[derive(ArgEnum, PartialEq, Debug, Clone)]
pub enum TargetFormat {
    Docker,
    Oci,
}

pub enum SourceType {
    Dockerfile { path: String },
    Cmd { tag: String },
}

impl FromStr for SourceType {
    type Err = anyhow::Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        let potion = arg.chars().position(|c| c == ':')
            .ok_or(Error::msg("error source"))?;
        let source_type = &arg[..potion];
        Ok(match source_type {
            "dockerfile" => SourceType::Dockerfile { path: arg[potion + 1..].to_string() },
            "cmd" => SourceType::Cmd { tag: arg[potion + 1..].to_string() },
            _ => return Err(Error::msg(format!("unknown source type: {}", source_type)))
        })
    }
}

pub enum TargetType {
    Registry(String),

}

impl FromStr for TargetType {
    type Err = anyhow::Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        let potion = arg.chars().position(|c| c == ':')
            .ok_or(Error::msg("error source"))?;
        let target_type = &arg[..potion];
        Ok(match target_type {
            "registry" => TargetType::Registry(arg[potion + 1..].to_string()),
            _ => return Err(Error::msg(format!("unknown target type: {}", target_type)))
        })
    }
}

pub struct BaseAuth {
    pub username: String,
    pub password: String,
}

impl FromStr for BaseAuth {
    type Err = anyhow::Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        let potion = arg.chars().position(|c| c == ':')
            .ok_or(Error::msg("error auth input"))?;
        Ok(BaseAuth { username: (&arg)[..potion].to_string(), password: (&arg)[potion + 1..].to_string() })
    }
}
