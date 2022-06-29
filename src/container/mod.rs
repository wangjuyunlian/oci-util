use crate::distribution::pull::pull;
use crate::filesystem::FileSystem;
use crate::image::config::ConfigFile;
use crate::image::layer::tar_file::TarFileTy;
use crate::image::Repositories;
use crate::util::get_sha256_digest;
use anyhow::{anyhow, Error, Result};
use log::{debug, info, warn};
use oci_distribution::secrets::RegistryAuth;
use oci_distribution::Reference;
use std::fs::File;
use std::path::PathBuf;

///
/// 初始化镜像
/// 如果本地不存在该镜像，则先pull再初始化。
pub async fn init(image: &Reference, auth: &RegistryAuth, force: bool) -> Result<Container> {
    // 判断是否已存在该容器：
    // 读取config
    // 读取layer
    // 初始化容器的文件系统
    let repo = Repositories::init()?;
    let config_digest = match repo.image_digest(&image) {
        Some(digest) => digest.to_string(),
        None => {
            info!("本地未找到镜像{:?}，先拉取镜像！", image);
            pull(image, auth).await?
        }
    };
    let path = FileSystem.container()?.join(&config_digest);
    let config = ConfigFile::load(&config_digest)?;

    let container = Container { path, config };
    if force {
        container.clear()?;
    } else {
        if container.exists() {
            debug!("local disk had a container");
            return Ok(container);
        }
    }
    container.init()?;
    Ok(container)
}

pub struct Container {
    pub config: ConfigFile,
    pub path: PathBuf,
}

impl Container {
    pub fn cmd(&self) -> PathBuf {
        let path = self.path.join(self.config.cmd.as_str());
        debug!("base={:?} path={:?}", self.path, path);
        path
    }
    pub fn exists(&self) -> bool {
        self.path.exists()
    }
    pub fn clear(&self) -> Result<()> {
        if self.exists() {
            std::fs::remove_dir_all(&self.path)?;
        }
        Ok(())
    }
    pub fn init(&self) -> Result<()> {
        let config = &self.config;
        std::fs::create_dir_all(&self.path)?;
        for copy in config.rootf.diff_ids.as_slice() {
            debug!("read layer {:?}", copy);
            let file = std::fs::File::open(
                FileSystem
                    .layer_blobs()?
                    .join(get_sha256_digest(copy.as_str())?),
            )?;
            let mut archive = tar::Archive::new(file);
            let entries = archive.entries().unwrap();
            for (_index, item) in entries.enumerate() {
                if let Ok(item) = item {
                    if let Some(path) = item.path()?.to_str().and_then(|x| Some(x.to_string())) {
                        let tar_file: TarFileTy = path.into();
                        apply_tar_file(tar_file, &self.path, item)?;
                    } else {
                        warn!("archive.entries.item has not path")
                    }
                } else {
                    warn!("archive.entries.item fail")
                }
            }
        }
        Ok(())
    }
}

pub fn apply_tar_file(
    tar_file_ty: TarFileTy,
    base: &PathBuf,
    mut item: tar::Entry<File>,
) -> Result<()> {
    debug!("{:?}", tar_file_ty);
    match tar_file_ty {
        TarFileTy::Delete(file) => {
            let target_path = base.join(&file);
            debug!("target: {:?}", target_path);
            if item.header().entry_type().is_file() {
                std::fs::remove_file(&target_path)?;
            } else if item.header().entry_type().is_dir() {
                std::fs::remove_dir_all(&target_path)?;
            } else {
                warn!("暂不支持其他文件类型: {:?}", item.header().entry_type());
            }
        }
        TarFileTy::Update(file) => {
            let target_path = base.join(&file);
            debug!("target: {:?}", target_path,);
            if item.header().entry_type().is_file() {
                let mut file = std::fs::File::create(target_path)?;
                // let mut data = Vec::with_capacity(item.size() as usize);
                // let read_size = item.read_to_end(&mut data)?;
                std::io::copy(&mut item, &mut file)?;
            } else if item.header().entry_type().is_dir() {
                std::fs::create_dir_all(target_path)?;
            } else {
                warn!("暂不支持其他文件类型: {:?}", item.header().entry_type());
            }
        }
    }
    Ok(())
}

impl TryFrom<&Reference> for Container {
    type Error = Error;

    fn try_from(image: &Reference) -> std::result::Result<Self, Self::Error> {
        let repo = Repositories::init()?;
        let config_digest = repo
            .image_digest(&image)
            .ok_or(anyhow!("本地未找到镜像{:?}", image))?;
        let path = FileSystem.container()?.join(&config_digest);
        let config = ConfigFile::load(&config_digest)?;
        Ok(Self { path, config })
    }
}
