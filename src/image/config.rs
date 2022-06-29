use crate::filesystem::FileSystem;
use crate::image::build::config::instructions::Kind;
use crate::image::build::config::BuildConfig;
use crate::image::layer::LayerAndData;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

pub struct ConfigFileAndData {
    pub file: ConfigFile,
    pub data: Vec<u8>,
}
impl ConfigFileAndData {
    pub fn load(digest: &String) -> Result<Self> {
        let config_path = FileSystem.image_sha256()?.join(digest);
        let data = std::fs::read(&config_path)
            .with_context(|| format!("读取镜像config文件{:?}失败", config_path))?;
        let file: ConfigFile = serde_json::from_slice(&data)?;
        Ok(Self { file, data })
    }

    pub fn load_layer(&self) -> Result<Vec<LayerAndData>> {
        let mut layers = Vec::with_capacity(self.file.rootf.diff_ids.len());
        for desc_item in self.file.rootf.diff_ids.iter() {
            let layer =
                LayerAndData::load(desc_item).context(anyhow!("加载layer：{}失败", desc_item))?;
            layers.push(layer);
        }
        Ok(layers)
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
        let config_path = FileSystem.image_sha256()?.join(digest);
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
