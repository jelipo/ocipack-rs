use anyhow::Result;
use crate::adapter::docker::DockerfileAdapter;

use crate::config::cmd::{BuildArgs, SourceType};

pub struct BuildCommand<'a> {
    pub build_args: &'a BuildArgs,
}

impl<'a> BuildCommand<'a> {
    pub fn build(&self) -> Result<()> {
        let _adapter = self.build_from_info()?;
        Ok(())
    }

    fn build_from_info(&self) -> Result<DockerfileAdapter> {
        match &self.build_args.source {
            SourceType::Dockerfile { path } => {
                DockerfileAdapter::new(&path, self.build_args.source_auth.as_ref())
            }
            SourceType::Cmd { tag: _ } => { todo!() }
        }
    }
}

