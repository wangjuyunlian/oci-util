use crate::filesystem::FileSystem;
use crate::image::Repositories;
use crate::util::DigestPre;
use anyhow::{Context, Result};
use log::{debug, warn};
use oci_distribution::secrets::RegistryAuth;
use oci_distribution::{Client, Reference};
use sha256::digest_bytes;

pub async fn pull(image: &Reference, auth: &RegistryAuth) -> Result<String> {
    // pull镜像清单
    // pull镜像的config
    // pull layer
    let client_config = oci_distribution::client::ClientConfig {
        protocol: oci_distribution::client::ClientProtocol::Https,
        ..Default::default()
    };
    let mut client = Client::new(client_config);
    let (manifest, _digest) = client.pull_image_manifest(&image, &auth).await?;

    let config_digest = manifest.config.digest.get_digest()?;
    if !FileSystem.exist_config(&config_digest)? {
        debug!("config[{}] is pulling……", config_digest);
        let mut out = Vec::new();
        client
            .pull_blob(&image, &manifest.config.digest, &mut out)
            .await
            .context("pull config失败")?;
        FileSystem.save_config(&config_digest, out.as_slice())?;
    } else {
        debug!("config[{}] is found in local", manifest.config.digest)
    }

    for item in manifest.layers.iter() {
        let layer_digest = item.digest.get_digest()?;
        if !FileSystem.exist_layer(&layer_digest)? {
            debug!("layer[{}] is pulling……", layer_digest);
            let mut out = Vec::new();
            client
                .pull_blob(&image, &item.digest, &mut out)
                .await
                .context("pull layer失败")?;
            FileSystem.save_layer(&layer_digest, out.as_slice())?;
        } else {
            debug!("layer[{}] is found in local", layer_digest)
        }
    }
    //
    let manifest_data = serde_json::to_vec(&manifest)?;
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

    let mut repo = Repositories::init()?;
    repo.update_and_save(&image, manifest_digest.sha256_pre())?;

    Ok(manifest_digest)
}
