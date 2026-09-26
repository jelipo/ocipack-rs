use crate::adapter::{BuildInfo, CopyFile, ImageInfo};
use crate::const_data::DEFAULT_IMAGE_HOST;
use anyhow::{Result, anyhow};
use dockerfile_parser::{BreakableStringComponent, Dockerfile, Instruction, ShellOrExecExpr};
use log::{debug, warn};
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub struct DockerfileAdapter {}

impl DockerfileAdapter {
    pub async fn parse(path: &str) -> Result<(ImageInfo, BuildInfo)> {
        if !Path::new(path).exists() {
            return Err(anyhow!("Dockerfile not found:{}", path));
        }
        let mut dockerfile_file = File::open(path).await?;
        let mut str_body = String::new();
        let read_size = dockerfile_file.read_to_string(&mut str_body).await?;
        debug!("Dockerfile size: {:?}", read_size);
        Self::parse_from_str(&str_body)
    }

    pub fn parse_from_str(str_body: &str) -> Result<(ImageInfo, BuildInfo)> {
        let dockerfile = Dockerfile::parse(str_body)?;
        let stages = dockerfile.stages().stages;
        if stages.len() != 1 {
            return Err(anyhow!("only support one stage in Dockerfile"));
        }
        let mut label_map = HashMap::<String, String>::new();
        let mut from_image = None;
        let mut user = None;
        let mut workdir = None;
        let mut envs_map = HashMap::<String, String>::new();
        let mut cmd = None;
        let mut entrypoint = None;
        let mut copy_files = Vec::new();
        let mut ports: Vec<String> = Vec::new();
        for instruction in dockerfile.instructions {
            match instruction {
                Instruction::From(from) => {
                    from_image = Some(ImageInfo {
                        image_raw_name: Some(from.image.content),
                        image_host: from.image_parsed.registry.unwrap_or_else(|| DEFAULT_IMAGE_HOST.to_string()),
                        image_name: from.image_parsed.image,
                        reference: from
                            .image_parsed
                            .tag
                            .or(from.image_parsed.hash)
                            .or_else(|| Some(String::from("latest")))
                            .ok_or_else(|| anyhow!("can not found hash or tag"))?,
                    })
                }
                Instruction::Arg(_) | Instruction::Run(_) => {
                    warn!("un support ARG and RUN")
                }
                Instruction::Label(label_i) => {
                    for label in label_i.labels {
                        let _ = label_map.insert(label.name.content, label.value.content);
                    }
                }
                Instruction::Entrypoint(value) => entrypoint = Some(command_from_expr(value.expr)),
                Instruction::Cmd(cmd_i) => cmd = Some(command_from_expr(cmd_i.expr)),
                Instruction::Copy(copy) => {
                    if !copy.flags.is_empty() {
                        return Err(anyhow!("copy not support flag"));
                    };
                    copy_files.push(CopyFile {
                        source_path: copy.sources.into_iter().map(|str| str.content).collect::<Vec<String>>(),
                        dest_path: copy.destination.content,
                    });
                }
                Instruction::Env(env_i) => {
                    for mut env in env_i.vars {
                        envs_map.insert(
                            env.key.content,
                            match env.value.components.remove(0) {
                                BreakableStringComponent::String(string) => string.content,
                                BreakableStringComponent::Comment(comment) => comment.content,
                            },
                        );
                    }
                }
                Instruction::Misc(mut misc) => match misc.instruction.content.as_str() {
                    "USER" => {
                        if let BreakableStringComponent::String(str) = misc.arguments.components.remove(0) {
                            user = Some(str.content.trim().to_string())
                        }
                    }
                    "WORKDIR" => {
                        if let BreakableStringComponent::String(str) = misc.arguments.components.remove(0) {
                            workdir = Some(str.content.trim().to_string());
                        }
                    }
                    "EXPOSE" => {
                        if let BreakableStringComponent::String(ports_str) = misc.arguments.components.remove(0) {
                            for str in ports_str.content.split_whitespace() {
                                let expose = if str.ends_with("/tcp") || str.ends_with("/udp") {
                                    let _port_num = u16::from_str(&str[..str.len() - 4])?;
                                    str.to_string()
                                } else {
                                    format!("{}/tcp", u16::from_str(str)?)
                                };
                                ports.push(expose)
                            }
                        }
                    }
                    "VOLUME" => warn!("un support VOLUME"),
                    "ADD" => warn!("TODO , need support ADD"),
                    "MAINTAINER" => warn!("un support MAINTAINER"),
                    _ => warn!("unknown dockerfile field:{}", misc.instruction.content),
                },
            }
        }
        let image_info = from_image.ok_or_else(|| anyhow!("dockerfile must has a 'From'"))?;
        Ok((
            image_info,
            BuildInfo {
                labels: label_map,
                envs: envs_map,
                user,
                workdir,
                cmd,
                entrypoint,
                copy_files,
                ports: if ports.is_empty() { None } else { Some(ports) },
            },
        ))
    }
}

fn command_from_expr(expr: ShellOrExecExpr) -> Vec<String> {
    match expr {
        ShellOrExecExpr::Shell(shell) => {
            let mut parts = shell
                .components
                .into_iter()
                .map(|component| match component {
                    BreakableStringComponent::String(string) => string.content,
                    BreakableStringComponent::Comment(comment) => comment.content,
                })
                .collect::<Vec<String>>();
            parts.insert(0, "/bin/sh".to_string());
            parts.insert(1, "-c".to_string());
            parts
        }
        ShellOrExecExpr::Exec(exec) => exec.elements.into_iter().map(|element| element.content).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exec_entrypoint() -> Result<()> {
        let (_, build) = DockerfileAdapter::parse_from_str("FROM alpine:3\nENTRYPOINT [\"/bin/echo\", \"hello\"]")?;
        assert_eq!(build.entrypoint, Some(vec!["/bin/echo".to_string(), "hello".to_string()]));
        Ok(())
    }

    #[test]
    fn shell_commands_are_a_single_shell_argument() -> Result<()> {
        let (_, build) = DockerfileAdapter::parse_from_str("FROM alpine:3\nENTRYPOINT echo hello world\nCMD echo ready")?;
        assert_eq!(
            build.entrypoint,
            Some(vec!["/bin/sh".to_string(), "-c".to_string(), "echo hello world".to_string()])
        );
        assert_eq!(
            build.cmd,
            Some(vec!["/bin/sh".to_string(), "-c".to_string(), "echo ready".to_string()])
        );
        Ok(())
    }
}
