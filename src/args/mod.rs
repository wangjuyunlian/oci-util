use crate::image::build::config::BuildConfig;
use oci_distribution::Reference;

#[derive(Debug)]
pub struct BuildArgs {
    pub config: BuildConfig,
    pub image: Reference,
}
