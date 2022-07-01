use crate::filesystem::FileSystem;
use crate::image::build::config::instructions::Kind;
use crate::image::build::config::BuildConfig;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub struct ConfigFileAndData {
    pub file: ConfigFile,
    pub data: Vec<u8>,
}
impl ConfigFileAndData {
    pub fn load(digest: &str) -> Result<Self> {
        let config_path = FileSystem.config_sha256()?.join(digest);
        let data = std::fs::read(&config_path)
            .with_context(|| format!("读取镜像config文件{:?}失败", config_path))?;
        let file: ConfigFile = serde_json::from_slice(&data)?;
        Ok(Self { file, data })
    }
}

#[derive(Serialize, Deserialize)]
pub struct ConfigFile {
    pub kind: Kind,
    pub cmd: String,
    pub rootf: RootFs,
}

impl ConfigFile {
    pub fn load(digest: &String) -> Result<Self> {
        let config_path = FileSystem.config_sha256()?.join(digest);
        let data = std::fs::read(&config_path)
            .with_context(|| format!("读取镜像config文件{:?}失败", config_path))?;
        let file: ConfigFile = serde_json::from_slice(&data)?;
        Ok(file)
    }

    pub fn new(config: &BuildConfig, diff_ids: Vec<String>) -> Result<Self> {
        let regix = regex::Regex::new("^/")?;
        let cmd = regix.replace(config.cmd.orgin.as_str(), "").to_string();
        Ok(Self {
            kind: config.kind.clone(),
            cmd: cmd,
            rootf: RootFs {
                typ: "layers".to_string(),
                diff_ids,
            },
        })
    }
    pub fn data(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self)?)
    }
}

#[derive(Serialize, Deserialize)]
pub struct RootFs {
    #[serde(rename = "type")]
    pub typ: String,
    pub diff_ids: Vec<String>,
}
