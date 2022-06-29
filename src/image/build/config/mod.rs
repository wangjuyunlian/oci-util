use crate::image::build::config::instructions::{Copy, Dest, Kind};
use anyhow::bail;
use log::warn;

pub mod instructions;
#[derive(Debug)]
pub struct BuildConfig {
    pub kind: Kind,
    pub copys: Vec<Copy>,
    pub cmd: Dest,
}

#[derive(Default)]
pub struct BuildConfigBuilder {
    pub kind: Option<Kind>,
    pub copys: Vec<Copy>,
    pub cmd: Option<Dest>,
}

impl BuildConfigBuilder {
    pub fn build(self) -> anyhow::Result<BuildConfig> {
        if let Some(cmd) = self.cmd {
            if let Some(kind) = self.kind {
                Ok(BuildConfig {
                    cmd,
                    kind,
                    copys: self.copys,
                })
            } else {
                bail!("配置项KIND缺失");
            }
        } else {
            bail!("配置项CMD缺失");
        }
    }
    pub fn append_copy(&mut self, copy: Copy) {
        self.copys.push(copy);
    }
    pub fn mut_cmd(&mut self, cmd: Dest) {
        if self.cmd.is_some() {
            warn!("Cmd重复配置！");
        }
        let _ = self.cmd.insert(cmd);
    }
    pub fn mut_kind(&mut self, kind: Kind) {
        if self.kind.is_some() {
            warn!("Kind重复配置！");
        }
        let _ = self.kind.insert(kind);
    }
}
