use anyhow::{bail, Error};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
pub enum Instruction {
    Kind(Kind),
    Copy(Copy),
    Cmd(Dest),
}
#[derive(Clone, Debug)]
pub struct Copy(pub PathBuf, pub Dest);
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Kind {
    Wasi,
    App,
}
#[derive(Debug, Clone)]
pub struct Dest {
    pub orgin: String,
    pub directory: Option<String>,
    pub file_name: Option<String>,
}

impl Dest {
    pub fn path_by_base(&self, mut base: PathBuf) -> PathBuf {
        if let Some(dir) = self.directory.as_ref() {
            base = base.join(dir)
        }
        if let Some(dir) = self.file_name.as_ref() {
            base = base.join(dir)
        }
        base
    }
}

impl TryFrom<String> for Dest {
    type Error = Error;

    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        let file_name_reg = regex::Regex::new(r"(.*)/([^/]*)$").unwrap();
        if let Some(res) = file_name_reg.captures(value.as_str()) {
            if let Some(file_name_match) = res.get(2) {
                let file_name = if file_name_match.as_str() == "" {
                    None
                } else {
                    Some(file_name_match.as_str().to_string())
                };
                if let Some(directory_match) = res.get(1) {
                    let directory = if directory_match.as_str() == "" {
                        None
                    } else {
                        let dir_reg = regex::Regex::new(r"/(.*)$").unwrap();
                        if let Some(res) = dir_reg.captures(directory_match.as_str()) {
                            if let Some(dir_match) = res.get(1) {
                                if dir_match.as_str() == "" {
                                    None
                                } else {
                                    Some(dir_match.as_str().to_string())
                                }
                            } else {
                                bail!("非法目标文件: {:?}", value);
                            }
                        } else {
                            bail!("非法目标文件: {:?}", value);
                        }
                    };
                    Ok(Self {
                        orgin: value,
                        directory,
                        file_name,
                    })
                } else {
                    bail!("非法目标文件: {:?}", value);
                }
            } else {
                bail!("非法目标文件: {:?}", value);
            }
        } else {
            bail!("非法目标文件: {:?}", value);
        }
    }
}

// pub enum Instruction {
//     From(FromInstruction),
//     Arg(ArgInstruction),
//     Label(LabelInstruction),
//     Run(RunInstruction),
//     Entrypoint(EntrypointInstruction),
//     Cmd(CmdInstruction),
//     Copy(CopyInstruction),
//     Env(EnvInstruction),
//     Misc(MiscInstruction)
// }
