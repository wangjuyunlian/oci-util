use anyhow::{bail, Result};
use async_recursion::async_recursion;
use log::{debug, warn};
use std::path::PathBuf;

pub fn get_sha256_digest(digest: &str) -> Result<String> {
    let regex = regex::Regex::new("^sha256:(.*)$")?;
    if let Some(capts) = regex.captures(digest) {
        if let Some(digest) = capts.get(1) {
            return Ok(digest.as_str().to_string());
        }
    }
    bail!("unreache!")
}

pub async fn copy_dir(from: PathBuf, dest: PathBuf) -> Result<()> {
    if !from.exists() || from.is_file() {
        bail!("源文件夹不存在或非文件夹");
    }
    if dest.exists() && dest.is_file() {
        bail!("目标文件夹为文件");
    }
    let num = copy_dir_detail(from.clone(), dest.clone()).await?;
    debug!("copy {} files: {:?} -> {:?}", num, from, dest);
    Ok(())
}
#[async_recursion]
async fn copy_dir_detail(from: PathBuf, dest: PathBuf) -> Result<usize> {
    if !dest.exists() {
        tokio::fs::create_dir_all(&dest).await?;
    }
    let mut dir = tokio::fs::read_dir(&from).await?;
    let mut cp_file_task = Vec::new();
    let mut cp_dir_task = Vec::new();
    let mut file_num = 0;
    while let Ok(entry) = dir.next_entry().await {
        if let Some(entry) = entry {
            if let Ok(metadata) = entry.metadata().await {
                let src_file = from.join(entry.file_name());
                let dest_file = dest.join(entry.file_name());
                if metadata.is_file() {
                    cp_file_task.push(tokio::spawn(tokio::fs::copy(src_file, dest_file)));
                } else {
                    cp_dir_task.push(tokio::spawn(copy_dir_detail(src_file, dest_file)));
                }
            }
        } else {
            break;
        }
    }
    for task in cp_file_task {
        match task.await {
            Ok(res) => {
                if let Err(e) = res {
                    warn!("{:?}", e);
                } else {
                    file_num += 1;
                }
            }
            Err(e) => {
                warn!("{:?}", e);
            }
        }
    }
    for task in cp_dir_task {
        match task.await {
            Ok(res) => match res {
                Err(e) => {
                    warn!("{:?}", e);
                }
                Ok(num) => {
                    file_num += num;
                }
            },
            Err(e) => {
                warn!("{:?}", e);
            }
        }
    }

    Ok(file_num)
}
