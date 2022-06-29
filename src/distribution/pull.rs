use crate::filesystem::FileSystem;
use crate::image::Repositories;
use crate::util::get_sha256_digest;
use anyhow::{Context, Result};
use log::debug;
use oci_distribution::secrets::RegistryAuth;
use oci_distribution::{Client, Reference};

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

    let config_digest = get_sha256_digest(manifest.config.digest.as_str())?;
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
        let layer_digest = get_sha256_digest(item.digest.as_str())?;
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
    let mut repo = Repositories::init()?;
    repo.update_and_save(&image, config_digest.clone())?;

    Ok(config_digest)
}
