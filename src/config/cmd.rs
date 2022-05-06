use std::str::FromStr;

use anyhow::Error;
use clap::Parser;

#[derive(Parser)]
#[clap(about = "An image tool", version, author = "jelipo <me@jelipo.com>")]
pub enum CmdArgs {
    /// 构建一个新的Image
    Build(BuildCmdArgs),
    /// 转换Image的格式，目前支持 Docker(V2S2) 和 OCI 的互相转换
    Transform,
}

#[derive(clap::Args)]
pub struct BuildCmdArgs {
    /// Allow insecure registry
    #[clap(long, short, parse(from_flag))]
    pub allow_insecure: bool,

    /// Allow target insecure registry
    #[clap(long, parse(from_flag))]
    pub target_allow_insecure: bool,

    /// Source type.
    /// Support dockerfile,cmd type
    /// Example:'dockerfile:/path/to/.Dockerfile','cmd:my.reg.com/source/image:1.0'
    #[clap(long, short)]
    pub source: SourceType,

    /// [OPTION] Auth of pull source image. Example:'myname:mypass','myname:${MY_PASSWORD_ENV}'
    #[clap(long)]
    pub source_auth: Option<BaseAuth>,

    /// Target type.
    /// Support 'registry'
    /// Example:'registry:my.reg.com/target/image:1.1'
    #[clap(long, short)]
    pub target: TargetType,

    /// [OPTION] Auth of push target image. Example:'myname:mypass','myname:${MY_PASSWORD_ENV}'
    #[clap(long)]
    pub target_auth: Option<BaseAuth>,

    /// [OPTION] Target format type. Support 'docker' and 'oci'.
    #[clap(long, short, default_value = "docker")]
    pub format: TargetFormat,
}

#[derive(Clone)]
pub enum TargetFormat {
    Docker,
    Oci,
}

impl FromStr for TargetFormat {
    type Err = anyhow::Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        Ok(match arg {
            "docker" => TargetFormat::Docker,
            "oci" => TargetFormat::Oci,
            _ => return Err(Error::msg(format!("unknown target format type: {}", arg))),
        })
    }
}

pub enum SourceType {
    Dockerfile { path: String },
    Cmd { tag: String },
}

impl FromStr for SourceType {
    type Err = anyhow::Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        let potion = arg.chars().position(|c| c == ':').ok_or_else(|| Error::msg("error source"))?;
        let source_type = &arg[..potion];
        Ok(match source_type {
            "dockerfile" => SourceType::Dockerfile {
                path: arg[potion + 1..].to_string(),
            },
            "cmd" => SourceType::Cmd {
                tag: arg[potion + 1..].to_string(),
            },
            _ => return Err(Error::msg(format!("unknown source type: {}", source_type))),
        })
    }
}

pub enum TargetType {
    Registry(String),
}

impl FromStr for TargetType {
    type Err = anyhow::Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        let potion = arg.chars().position(|c| c == ':').ok_or_else(|| Error::msg("error source"))?;
        let target_type = &arg[..potion];
        Ok(match target_type {
            "registry" => TargetType::Registry(arg[potion + 1..].to_string()),
            _ => return Err(Error::msg(format!("unknown target type: {}", target_type))),
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
        let potion = arg.chars().position(|c| c == ':').ok_or_else(|| Error::msg("error auth input"))?;
        Ok(BaseAuth {
            username: arg[..potion].to_string(),
            password: arg[potion + 1..].to_string(),
        })
    }
}
