use crate::image::config::ConfigFileAndData;
use crate::image::Repositories;
use anyhow::{anyhow, Result};
use log::debug;
use oci_distribution::client::{Config, ImageLayer};
use oci_distribution::secrets::RegistryAuth;
use oci_distribution::{manifest, Client, Reference};
use std::collections::HashMap;

pub async fn push(image: &Reference, auth: &RegistryAuth) -> Result<()> {
    // 读取images.json
    // 读取镜像的config、layer
    // 拼接镜像清单
    // push
    debug!("开始查找本地镜像……");
    let repo = Repositories::init()?;
    let config_digest = repo
        .image_digest(&image)
        .ok_or(anyhow!("本地未找到镜像{:?}", image))?;
    debug!("加载镜像config文件……");
    let config = ConfigFileAndData::load(config_digest)?;
    debug!("加载镜像layer文件……");
    let layers = config.load_layer()?;
    let ConfigFileAndData { file: _, data } = config;
    let annotations: Option<HashMap<String, String>> = Some(HashMap::new());

    let layers: Vec<ImageLayer> = layers.into_iter().map(|x| x.into()).collect();

    let config = Config {
        data: data,
        media_type: manifest::IMAGE_CONFIG_MEDIA_TYPE.to_string(),
        annotations: None,
    };

    let image_manifest = manifest::OciImageManifest::build(&layers, &config, annotations);
    debug!("{}", image_manifest);
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
