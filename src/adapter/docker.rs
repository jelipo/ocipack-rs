use std::collections::HashMap;
use std::fs::File;
use std::io::Read;

use anyhow::{Error, Result};
use dockerfile_parser::{BreakableStringComponent, Dockerfile, Instruction, ShellOrExecExpr};
use log::warn;

use crate::adapter::{Adapter, FromImageAdapter};
use crate::config::{BaseImage, RegAuthType};
use crate::config::cmd::BaseAuth;

pub struct DockerfileAdapter {
    docker_file_path: String,
    info: DockerfileInfo,
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
            return Err(Error::msg("only support one stage in Dockerfile"));
        }
        let mut label_map = HashMap::<String, String>::new();
        let mut from_image = None;
        let mut user = None;
        let mut workdir = None;
        for instruction in dockerfile.instructions {
            println!("{:?}", instruction);
            match instruction {
                Instruction::From(from) => from_image = Some(FromImage {
                    image_host: from.image_parsed.registry,
                    image_name: from.image_parsed.image,
                    reference: from.image_parsed.tag.or(from.image_parsed.hash)
                        .ok_or(Error::msg("can not found hash or tag"))?,
                }),
                Instruction::Arg(_) | Instruction::Run(_) => {
                    warn!("un support ARG and RUN")
                }
                Instruction::Label(label_i) => for label in label_i.labels {
                    let _ = label_map.insert(label.name.content, label.value.content);
                },
                Instruction::Entrypoint(entrypoint) => match &entrypoint.expr {
                    ShellOrExecExpr::Shell(_shell) => {}
                    ShellOrExecExpr::Exec(_exec) => {}
                },
                Instruction::Cmd(_cmd) => {}
                Instruction::Copy(_copy) => {}
                Instruction::Env(_env) => {}
                Instruction::Misc(mut misc) => match misc.instruction.content.as_str() {
                    "USER" => match misc.arguments.components.remove(0) {
                        BreakableStringComponent::String(str) => user = Some(str.content.trim().to_string()),
                        _ => {}
                    }
                    "WORKDIR" => match misc.arguments.components.remove(0) {
                        BreakableStringComponent::String(str) => workdir = Some(str.content.trim().to_string()),
                        _ => {}
                    }
                    "EXPOSE" => { todo!() }
                    "VOLUME" => {}
                    "ADD" => {}
                    "MAINTAINER" => warn!("un support MAINTAINER"),
                    _ => warn!("unknown dockerfile field:{}",misc.instruction.content)
                }
            }
        }

        Ok(DockerfileAdapter {
            docker_file_path: "".to_string(),
            info: DockerfileInfo {
                from_info: from_image.ok_or(Error::msg("dockerfile must has a 'From'"))?,
                labels: label_map,
                user,
                workdir,
            },
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


struct DockerfileInfo {
    from_info: FromImage,
    labels: HashMap<String, String>,
    user: Option<String>,
    workdir: Option<String>,
}

struct FromImage {
    image_host: Option<String>,
    image_name: String,
    reference: String,
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