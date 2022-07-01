use crate::image::config::ConfigFileAndData;
use crate::image::manifest::{load_layer, Manifest};
use crate::image::Repositories;
use crate::util::DigestPre;
use anyhow::{anyhow, Result};
use log::debug;
use oci_distribution::client::{Config, ImageLayer};
use oci_distribution::secrets::RegistryAuth;
use oci_distribution::{manifest, Client, Reference};

pub async fn push(image: &Reference, auth: &RegistryAuth) -> Result<()> {
    // 读取images.json
    // 读取镜像的config、layer
    // 拼接镜像清单
    // push
    debug!("开始查找本地镜像……");
    let repo = Repositories::init()?;
    let manifest_digest = repo
        .image_digest(&image)
        .ok_or(anyhow!("本地未找到镜像{:?}", image))?
        .get_digest()?;
    debug!("");
    let image_manifest = Manifest::load(manifest_digest.as_str())?.to_oci_manifest()?;

    debug!("加载镜像config文件……");
    let config = ConfigFileAndData::load(&image_manifest.config.digest.get_digest()?)?;
    let ConfigFileAndData { file: _, data } = config;

    debug!("加载镜像layer文件……");
    let layers = load_layer(&image_manifest.layers)?;
    let layers: Vec<ImageLayer> = layers.into_iter().map(|x| x.into()).collect();

    let config = Config {
        data: data,
        media_type: manifest::IMAGE_CONFIG_MEDIA_TYPE.to_string(),
        annotations: None,
    };

    let client_config = oci_distribution::client::ClientConfig {
        protocol: oci_distribution::client::ClientProtocol::Https,
        ..Default::default()
    };
    let mut client = Client::new(client_config);
    let _response = client
        .push(&image, &layers, config, &auth, Some(image_manifest))
        .await
        .map(|push_response| push_response.manifest_url)
        .expect("Cannot push");
    Ok(())
}
