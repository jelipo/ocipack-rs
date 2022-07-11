use std::env;
use std::str::FromStr;

use anyhow::Result;
use anyhow::{anyhow, Error};
use clap::Parser;
use url::Url;

use crate::reg::proxy::{ProxyAuth, ProxyInfo};

#[derive(Parser)]
#[clap(about = "Fast build docker/oci image", version, author = "jelipo(github.com/jelipo)")]
pub enum CmdArgs {
    /// Build a new image and push to registry.
    Build(Box<BuildCmdArgs>),
    /// 转换Image的格式，目前支持 Docker(V2S2) 和 OCI 的互相转换
    Transform,
    /// Clean cache dir
    Clean(CleanCmdArgs),
}

#[derive(clap::Args)]
pub struct CleanCmdArgs {
    /// Clean all file.
    #[clap(long, short, parse(from_flag))]
    pub all: bool,

    /// Clean temp dir.
    #[clap(long, short, parse(from_flag))]
    pub temp: bool,
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

    /// [OPTION] Proxy of pull source image. Example:'socks5://127.0.0.1:1080','http://name:pass@example:8080'
    #[clap(long)]
    pub source_proxy: Option<ProxyInfo>,

    /// Target type.
    /// Support 'registry'
    /// Example:'registry:my.reg.com/target/image:1.1'
    #[clap(long, short)]
    pub target: TargetType,

    /// [OPTION] Auth of push target image. Example:'myname:mypass','myname:${MY_PASSWORD_ENV}'
    #[clap(long)]
    pub target_auth: Option<BaseAuth>,

    /// [OPTION] Proxy of push target image. Example:'socks5://127.0.0.1:1080','http://name:pass@example:8080'
    #[clap(long)]
    pub target_proxy: Option<ProxyInfo>,

    /// [OPTION] Target format type. Support 'docker' and 'oci'.
    #[clap(long, short, default_value = "docker")]
    pub format: TargetFormat,

    /// [OPTION] Connection timeout in seconds.
    #[clap(long, default_value = "600")]
    pub conn_timeout: u64,

    /// [OPTION] Compress files using zstd.
    #[clap(long, parse(from_flag))]
    pub use_zstd: bool,
}

#[derive(Clone)]
pub enum TargetFormat {
    Docker,
    Oci,
}

impl FromStr for TargetFormat {
    type Err = Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        Ok(match arg {
            "docker" => TargetFormat::Docker,
            "oci" => TargetFormat::Oci,
            _ => return Err(anyhow!("unknown target format type: {}", arg)),
        })
    }
}

pub enum SourceType {
    Dockerfile { path: String },
    Cmd { tag: String },
}

impl FromStr for SourceType {
    type Err = Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        let potion = arg.chars().position(|c| c == ':').ok_or_else(|| anyhow!("error source"))?;
        let source_type = &arg[..potion];
        Ok(match source_type {
            "dockerfile" => SourceType::Dockerfile {
                path: arg[potion + 1..].to_string(),
            },
            "cmd" => SourceType::Cmd {
                tag: arg[potion + 1..].to_string(),
            },
            _ => return Err(anyhow!("unknown source type: {}", source_type)),
        })
    }
}

impl FromStr for ProxyInfo {
    type Err = Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(arg)?;
        let auth_opt = if !url.username().eq("") {
            Some(ProxyAuth::new(
                url.username().to_string(),
                url.password().unwrap_or("").to_string(),
            ))
        } else {
            None
        };
        let addr = format!(
            "{}://{}:{}",
            url.scheme(),
            url.host_str().unwrap_or("127.0.0.1"),
            url.port().unwrap_or(80)
        );
        Ok(ProxyInfo::new(addr, auth_opt))
    }
}

pub enum TargetType {
    Registry(String),
}

impl FromStr for TargetType {
    type Err = Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        let potion = arg.chars().position(|c| c == ':').ok_or_else(|| anyhow!("error source"))?;
        let target_type = &arg[..potion];
        Ok(match target_type {
            "registry" => TargetType::Registry(arg[potion + 1..].to_string()),
            _ => return Err(anyhow!("unknown target type: {}", target_type)),
        })
    }
}

pub struct BaseAuth {
    pub username: String,
    pub password: String,
}

impl FromStr for BaseAuth {
    type Err = Error;

    fn from_str(arg: &str) -> Result<Self, Self::Err> {
        let potion = arg.chars().position(|c| c == ':').ok_or_else(|| anyhow!("error auth input"))?;
        Ok(BaseAuth {
            username: value_or_env(&arg[..potion])?,
            password: value_or_env(&arg[potion + 1..])?,
        })
    }
}

fn value_or_env(param: &str) -> Result<String> {
    let value = if param.starts_with("${") && param.ends_with('}') {
        env::var(&param[2..param.len() - 1])?
    } else {
        param.to_string()
    };
    Ok(value)
}

#[test]
fn it_works() -> Result<()> {
    println!("{:?}", value_or_env("${PATH}")?);
    Ok(())
}
