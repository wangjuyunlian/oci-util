pub mod tar_file;

use crate::filesystem::FileSystem;
use crate::util::DigestPre;
use anyhow::{anyhow, Context, Result};
use oci_distribution::client::ImageLayer;

pub static LAYER_MEDIA_TYPE: &str = "application/vnd.oci.image.layer.v1.tar";

pub struct LayerAndData {
    pub data: Vec<u8>,
    // pub layer: Layer,
    pub media_type: String,
}

// #[derive(Serialize, Deserialize)]
// pub struct Layer {
//     desc: Descriptor,
// }
//
// impl Layer {
//     /// 好像不行
//     pub fn save_descriptor(&self) -> Result<String> {
//         let path = FileSystem.layer_contents()?;
//         let data = serde_json::to_vec(&self.desc)?;
//         let digest = sha256::digest_bytes(data.as_slice());
//         let path = path.join(&digest);
//         std::fs::write(path, data)?;
//         Ok(digest)
//     }
// }
//
// impl From<Descriptor> for Layer {
//     fn from(desc: Descriptor) -> Self {
//         Self { desc }
//     }
// }
// impl Deref for Layer {
//     type Target = Descriptor;
//
//     fn deref(&self) -> &Self::Target {
//         &self.desc
//     }
// }

impl LayerAndData {
    pub fn load(desc_digest: &String, media_type: String) -> Result<Self> {
        let layer_digest = desc_digest.get_digest()?;
        let layer_path = FileSystem.layer_blobs()?.join(&layer_digest);
        let data = std::fs::read(&layer_path).context(anyhow!("加载{:?}失败", layer_path))?;
        Ok(Self {
            data,
            media_type, // layer: desc.into(),
        })
    }
}

impl From<LayerAndData> for ImageLayer {
    fn from(lad: LayerAndData) -> Self {
        let LayerAndData { data, media_type } = lad;
        // let media_type = layer.desc.media_type().to_string();
        Self {
            data,
            media_type,
            annotations: None,
        }
    }
}
