use std::fs::File;
use std::io::Read;
use anyhow::{Error, Result};
use dockerfile_parser::{Dockerfile, Instruction, ShellOrExecExpr};

use crate::adapter::{Adapter, FromImageAdapter};
use crate::config::{BaseImage, RegAuthType};
use crate::config::cmd::BaseAuth;

pub struct DockerfileAdapter {
    docker_file_path: String,

    image_host: String,
    image_name: String,
    /// 可以是TAG或者digest
    reference: String,
    /// 验证方式
    auth_type: RegAuthType,
}

impl DockerfileAdapter {
    pub fn new(path: &str, auth: Option<&BaseAuth>) -> Result<DockerfileAdapter> {
        let mut dockerfile_file = File::open(path)?;
        let mut str_body = String::new();
        let _read_size = dockerfile_file.read_to_string(&mut str_body)?;
        let dockerfile = Dockerfile::parse(&str_body)?;
        let stages = dockerfile.stages().stages;
        if stages.len() != 1 {
            return Err(Error::msg("Only support one stage in Dockerfile"));
        }
        let stage = stages.get(0).expect("unknown error");
        for x in &stage.instructions {
            println!("{:?}", x);
            match x {
                Instruction::From(from) => {}
                Instruction::Arg(_) => {}
                Instruction::Label(s) => {}
                Instruction::Run(_) => {},
                Instruction::Entrypoint(entrypoint) => {
                    match entrypoint.expr {
                        ShellOrExecExpr::Shell(_) => {}
                        ShellOrExecExpr::Exec(_) => {}
                    }
                }
                Instruction::Cmd(cmd) => {

                }
                Instruction::Copy(copy) => {}
                Instruction::Env(env) => {}
                Instruction::Misc(misc) => {
                    //println!("misc: {:?}", misc);
                }
            }
        }

        Ok(DockerfileAdapter {
            docker_file_path: "".to_string(),
            image_host: "".to_string(),
            image_name: "".to_string(),
            reference: "".to_string(),
            auth_type: match auth {
                None => RegAuthType::LocalDockerAuth { reg_host: "".to_string() },
                Some(auth) => RegAuthType::CustomPassword {
                    username: auth.username.clone(),
                    password: auth.password.clone(),
                }
            },
        })
    }
}

impl Adapter for DockerfileAdapter {
    fn image_info(&self) -> Result<BaseImage> {
        Ok(BaseImage {
            use_https: true,
            reg_host: "".to_string(),
            image_name: "".to_string(),
            reference: "".to_string(),
            auth_type: self.auth_type.clone(),
        })
    }
}

impl FromImageAdapter for DockerfileAdapter {
    fn new_envs(&self) -> Option<&[String]> {
        todo!()
    }

    fn new_cmds(&self) -> Option<&[String]> {
        todo!()
    }
}