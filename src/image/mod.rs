pub mod build;
pub mod config;
pub mod layer;
pub mod manifest;

use crate::filesystem::FileSystem;
use anyhow::Result;
use log::warn;
use oci_distribution::Reference;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Serialize, Deserialize, Default)]
pub struct Repositories {
    #[serde(default)]
    repositories: HashMap<String, HashMap<String, String>>,
}

impl Repositories {
    /// 初始本地仓库信息（从本地读取信息文件）
    pub fn init() -> Result<Self> {
        let repos_path = FileSystem.images_json()?;
        Ok(std::fs::read(&repos_path)
            .and_then(
                |x| match serde_json::from_slice::<Repositories>(x.as_slice()) {
                    Ok(ins) => Ok(ins),
                    Err(e) => {
                        warn!("初始化images.json异常：{:?}", e);
                        Err(e.into())
                    }
                },
            )
            .unwrap_or(Repositories::default()))
    }

    /// 获取本地镜像的digest
    pub fn image_digest(&self, image: &Reference) -> Option<&String> {
        let full_name = full_name(image);
        let whole_name = image.whole();
        if let Some(repo) = self.repositories.get(&full_name) {
            repo.get(&whole_name)
        } else {
            None
        }
    }
    /// 更新镜像信息
    pub fn update(&mut self, image: &Reference, digest: String) {
        let full_name = full_name(image);
        let whole_name = image.whole();

        if let Some(repo) = self.repositories.get_mut(&full_name) {
            repo.insert(whole_name, digest);
        } else {
            let mut repo = HashMap::new();
            repo.insert(whole_name, digest);
            self.repositories.insert(full_name, repo);
        }
    }
    /// 更新镜像信息、并保存至本地
    pub fn update_and_save(&mut self, image: &Reference, digest: String) -> Result<()> {
        self.update(image, digest);
        self.save()
    }
    /// 保存至本地
    pub fn save(&self) -> Result<()> {
        let repos_path = FileSystem.images_json()?;
        std::fs::write(&repos_path, serde_json::to_vec(&self)?)?;
        Ok(())
    }
}

fn full_name(image: &Reference) -> String {
    if image.registry() == "" {
        image.repository().to_string()
    } else {
        format!("{}/{}", image.registry(), image.repository())
    }
}
