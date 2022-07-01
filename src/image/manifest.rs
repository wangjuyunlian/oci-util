use crate::filesystem::FileSystem;
use crate::image::layer::LayerAndData;
use anyhow::{anyhow, Context, Result};
use oci_distribution::manifest::{OciDescriptor, OciImageManifest};

pub struct Manifest(Vec<u8>);

impl Manifest {
    pub fn load(digest: &str) -> Result<Self> {
        let path = FileSystem.manifest_sha256()?.join(digest);
        let data = std::fs::read(path)?;
        Ok(Self(data))
    }
    pub fn to_oci_manifest(&self) -> Result<OciImageManifest> {
        Ok(serde_json::from_slice(&self.0)?)
    }
}

pub fn load_layer(lays_des: &Vec<OciDescriptor>) -> Result<Vec<LayerAndData>> {
    let mut layers = Vec::with_capacity(lays_des.len());
    for desc_item in lays_des.iter() {
        let layer = LayerAndData::load(&desc_item.digest, desc_item.media_type.clone())
            .context(anyhow!("加载layer：{}失败", desc_item))?;
        layers.push(layer);
    }
    Ok(layers)
}
