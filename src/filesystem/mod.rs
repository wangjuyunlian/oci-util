use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub mod snapshot;

pub struct FileSystem;

///
/// $HOME/hpmq
///       ├──imagedb
///       │  ├──images.json
///       │  ├──sha256
///       │  │  ├──image的config文件；文件名为文件的sha256摘要
///       ├──layerdb
///       │  ├──contents?（待实现）
///       │  │  │  layer的描述文件（Descriptor）；文件名为文件的sha256摘要
///       │  ├──blobs
///       │  │  ├──sha256
///       │  │  │  ├──image的layer文件
///       ├──containerdb
///       │  ├──image的config文件的sha256
///       │  │  ├──image展开后的文件目录
///
impl FileSystem {
    ///
    /// |Platform | Value                | Example        |
    /// | ------- | -------------------- | -------------- |
    /// | Linux   | `$HOME`              | /home/alice    |
    /// | macOS   | `$HOME`              | /Users/Alice   |
    /// | Windows | `{FOLDERID_Profile}` | C:\Users\Alice |
    pub fn home(&self) -> Result<PathBuf> {
        let path = dirs::home_dir()
            .and_then(|path| Some(path.join(".hpmq")))
            .ok_or(anyhow!("找不到HOME路径"))?;
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }
    pub fn layer(&self) -> Result<PathBuf> {
        let path = self.home()?.join("layerdb");
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }
    pub fn layer_contents(&self) -> Result<PathBuf> {
        let path = self.home()?.join("layerdb").join("contents");
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }
    pub fn layer_blobs(&self) -> Result<PathBuf> {
        let path = self.home()?.join("layerdb").join("blobs").join("sha256");
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }
    pub fn image_sha256(&self) -> Result<PathBuf> {
        let path = self.home()?.join("imagedb").join("sha256");
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }
    pub fn images_json(&self) -> Result<PathBuf> {
        let path = self.home()?.join("imagedb");
        std::fs::create_dir_all(&path)?;
        Ok(path.join("images.json"))
    }
    pub fn container(&self) -> Result<PathBuf> {
        let path = self.home()?.join("containerdb");
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    pub fn exist_config(&self, sha256_digest: &String) -> Result<bool> {
        let config_path = self.image_sha256()?;
        Ok(config_path.join(sha256_digest).exists())
    }
    pub fn exist_layer(&self, sha256_digest: &String) -> Result<bool> {
        let layer_path = self.layer_blobs()?;
        Ok(layer_path.join(sha256_digest).exists())
    }
    pub fn exist_container(&self, sha256_digest: &String) -> Result<bool> {
        let layer_path = self.container()?;
        Ok(layer_path.join(sha256_digest).exists())
    }

    pub fn save_config(&self, sha256_digest: &String, data: &[u8]) -> Result<()> {
        let config_path = self.image_sha256()?;
        Ok(std::fs::write(config_path.join(sha256_digest), data)?)
    }
    pub fn save_layer(&self, sha256_digest: &String, data: &[u8]) -> Result<()> {
        let config_path = self.layer_blobs()?;
        Ok(std::fs::write(config_path.join(sha256_digest), data)?)
    }
}
