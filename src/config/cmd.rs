use std::str::FromStr;

use clap::{ArgEnum, Parser, Subcommand};

#[derive(Parser)]
#[clap(about, version, author = "jelipo")]
pub enum CmdArgs {
    Build(BuildArgs)
}

#[derive(clap::Args)]
#[clap(about, author, version)]
pub struct BuildArgs {
    #[clap(subcommand)]
    from: FromType,
}

#[derive(Subcommand)]
#[clap(about, version, author)]
pub enum FromType {
    FromDockerfile(DockerfileArgs)
}

#[derive(clap::Args)]
#[clap(about, author, version)]
pub struct DockerfileArgs {
    path: String,
}


