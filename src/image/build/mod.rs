pub mod config;

use crate::args::BuildArgs;
use crate::filesystem::snapshot::Snapshot;
use crate::filesystem::FileSystem;
use crate::image::{config::ConfigFile, Repositories};
use crate::util::DigestPre;
use anyhow::{bail, Result};
use log::{debug, warn};
use oci_distribution::manifest::{OciDescriptor, OciImageManifest};
use oci_spec::image::MediaType;
use sha256::digest_bytes;

pub async fn build(args: &BuildArgs) -> Result<String> {
    debug!("开始构建任务: {:?}", args);
    // let docker_file = dockerfile_parser::Dockerfile::parse(
    //     std::fs::read_to_string(args.config.as_str())?.as_str(),
    // )?;
    let build_file = &args.config;
    debug!("    构建参数: {:?}", build_file);
    let snapshot_base = Snapshot::new()?;
    let mut snapshots = Vec::new();

    let mut before = snapshot_base.clone();

    for copy in build_file.copys.iter() {
        before = before.init_by_self().await?;
        before.copy_in(&copy.0, &copy.1)?;
        snapshots.push(before.clone());
    }
    if !before.file_exist(&build_file.cmd.path_by_base(before.path.clone())) {
        bail!("镜像构建失败：不存在CMD【{:?}】文件", build_file.cmd);
    }
    ////////////// 构建layer
    let mut descriptors = Vec::with_capacity(snapshots.len());
    let mut layer_descriptors = Vec::with_capacity(snapshots.len());

    let mut snapshot = snapshot_base;
    for tmp in snapshots {
        let next = tmp.into();
        let changeset = snapshot.diff(&next);
        debug!("changeset: {:?}", changeset);
        match changeset.write_layer(FileSystem.layer()?, &MediaType::ImageLayer) {
            Ok((_index, describe)) => {
                // let layer: Layer = describe.into();
                descriptors.push(describe.digest().to_string());

                layer_descriptors.push(OciDescriptor {
                    media_type: describe.media_type().to_string(),
                    digest: describe.digest().to_string(),
                    size: describe.size(),
                    urls: None,
                    annotations: None,
                });
            }
            Err(e) => {
                bail!("生成layer失败：{}", e);
            }
        }
        snapshot = next;
    }
    // 构建config、写入sha256文件夹
    let config = ConfigFile::new(&build_file, descriptors)?;
    let config_data = serde_json::to_vec(&config)?;
    let config_digest = digest_bytes(config_data.as_slice());
    let config_descriptor = OciDescriptor {
        media_type: MediaType::ImageConfig.to_string(),
        digest: config_digest.sha256_pre(),
        size: config_data.len() as i64,
        urls: None,
        annotations: None,
    };
    let config_path = FileSystem
        .config_sha256()
        .and_then(|x| {
            if let Err(e) = std::fs::create_dir_all(&x) {
                warn!("创建文件夹{:?}失败{:?}", x, e);
            }
            Ok(x)
        })?
        .join(config_digest.as_str());
    std::fs::write(config_path, config_data)?;
    // 构建manifest
    // let annotations: Option<HashMap<String, String>> = Some(HashMap::new());
    let image_manifest = OciImageManifest {
        schema_version: 2,
        media_type: Some(MediaType::ImageManifest.to_string()),
        config: config_descriptor,
        layers: layer_descriptors,
        annotations: None,
    };
    let manifest_data = serde_json::to_vec(&image_manifest)?;
    let manifest_digest = digest_bytes(manifest_data.as_slice());
    let manifest_path = FileSystem
        .manifest_sha256()
        .and_then(|x| {
            if let Err(e) = std::fs::create_dir_all(&x) {
                warn!("创建文件夹{:?}失败{:?}", x, e);
            }
            Ok(x)
        })?
        .join(manifest_digest.as_str());
    std::fs::write(manifest_path, manifest_data)?;

    // 更新images.json
    let mut repos = Repositories::init()?;
    repos.update_and_save(&args.image, manifest_digest.sha256_pre())?;
    Ok(manifest_digest)
}
