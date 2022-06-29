mod blob_writer;

use crate::filesystem::snapshot::blob_writer::BlobWriter;
use crate::image::build::config::instructions::Dest;
use crate::util::copy_dir;
use anyhow::{anyhow, bail, Context, Result};
use jwalk::WalkDirGeneric;
use log::{debug, error};
use oci_spec::image::{Descriptor, DescriptorBuilder, MediaType};

use chrono::Utc;
use seahash::SeaHasher;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::ffi::OsString;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::{fs, io};
use tempfile::tempdir;

#[derive(Clone)]
pub struct Snapshot {
    pub path: PathBuf,
    pub dest_dir: String,
}

impl Snapshot {
    pub fn init(path: PathBuf) -> Result<Self> {
        // let path = tempdir().context("无法创建临时文件夹")?.into_path();
        // debug!("Snapshot {:?}", path);
        let dest_dir = "/".to_string();
        Ok(Self { path, dest_dir })
    }
    pub fn new() -> Result<Self> {
        let path = tempdir().context("无法创建临时文件夹")?.into_path();
        debug!("Snapshot {:?}", path);
        Self::init(path)
    }
    pub async fn init_by_self(&self) -> Result<Self> {
        let path = tempdir().context("无法创建临时文件夹")?.into_path();
        // let dir = std::fs::read_dir(&self.path)?;
        debug!("Snapshot {:?}", path);
        copy_dir(self.path.clone(), path.clone()).await?;
        Self::init(path)
    }
    pub fn copy_in(&self, src: impl Into<PathBuf>, dst: &Dest) -> Result<()> {
        let src_path = src.into();
        let mut dst_path = self.path.clone();
        if let Some(dir) = &dst.directory {
            dst_path = dst_path.join(dir);
        }
        std::fs::create_dir_all(&dst_path)?;
        if let Some(file_name) = &dst.file_name {
            dst_path = dst_path.join(file_name)
        } else {
            dst_path = dst_path.join(
                src_path
                    .file_name()
                    .ok_or(anyhow!("获取源文件文件名失败"))?,
            )
        }
        debug!("{:?} -> {:?}", src_path, dst_path);
        std::fs::copy(&src_path, &dst_path).context("copy_in报错")?;
        Ok(())
    }
    pub fn file_exist(&self, file: impl Into<PathBuf>) -> bool {
        self.path.clone().join(file.into()).exists()
    }
    pub fn generate_path(&self, path: &String) -> PathBuf {
        self.path.join(path)
    }

    pub fn entries(&self) -> HashMap<String, SnapshotEntry> {
        let source_dir = self.path.clone();
        WalkDirGeneric::<((), SnapshotEntry)>::new(&source_dir)
            .skip_hidden(false)
            .process_read_dir(|_depth, _path, _read_dir_state, children| {
                children.iter_mut().flatten().for_each(|dir_entry| {
                    if !dir_entry.file_type.is_dir() {
                        dir_entry.client_state = SnapshotEntry::new(
                            &dir_entry.path(),
                            &dir_entry.file_type(),
                            dir_entry.metadata().ok(),
                        );
                    }
                })
            })
            .into_iter()
            .filter_map(|entry_result| match entry_result {
                Ok(entry) => {
                    let path = entry.path();

                    let relative_path = path
                        .strip_prefix(&source_dir)
                        .expect("Should always be able to strip the root dir");
                    match relative_path == PathBuf::from("") {
                        true => None, // This is the entry for the dir itself so ignore it
                        false => Some((
                            relative_path.to_string_lossy().to_string(), // Should be lossless on Linux (and MacOS)
                            entry.client_state,
                        )),
                    }
                }
                Err(error) => {
                    error!("While snapshotting `{}`: {}", source_dir.display(), error);
                    None
                }
            })
            .collect()
    }

    /// Create a set of changes by determining the difference between two snapshots
    pub fn diff(&self, new_other: &Snapshot) -> ChangeSet {
        let mut changes = Vec::new();
        for (path, entry) in self.entries().iter() {
            match new_other.entries().get(path) {
                Some(other_entry) => {
                    if entry != other_entry {
                        changes.push(Change::Modified(path.into()))
                    }
                }
                None => changes.push(Change::Removed(path.into())),
            }
        }
        for path in new_other.entries().keys() {
            if !self.entries().contains_key(path) {
                changes.push(Change::Added(path.into()))
            }
        }
        ChangeSet::new(
            new_other.path.clone(),
            new_other.dest_dir.clone().into(),
            changes,
        )
    }
}

#[derive(Debug)]
pub struct ChangeSet {
    /// The source directory, on the local filesystem, for the changes
    source_dir: PathBuf,
    /// The destination directory, within the image's root filesystem, for the changes
    dest_dir: PathBuf,
    /// The change items
    pub(crate) items: Vec<Change>,
}
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SnapshotEntry {
    /// Metadata on the file, directory, or symlink
    /// Should only be `None` if there was an error getting the metadata
    /// while creating the snapshot.
    metadata: Option<SnapshotEntryMetadata>,

    /// Hash of the content of the file
    /// Used to detect if the content of a file is changed.
    /// Will be `None` if the entry is a directory or symlink.
    fingerprint: Option<u64>,

    /// The target of the symlink
    /// Used to detect if the target of the symlink has changed.
    /// Will be `None` if the entry is a file or directory.
    target: Option<String>,
}

#[derive(Debug, PartialEq, Ord, PartialOrd, Eq)]
pub enum Change {
    Added(String),
    Modified(String),
    Removed(String),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotEntryMetadata {
    uid: u32,
    gid: u32,
    readonly: bool,
}
// impl From<Snapshot> for oci_images_util::snapshot::Snapshot {
//     fn from(val: Snapshot) -> Self {
//         oci_images_util::snapshot::Snapshot::new(val.path, "/")
//     }
// }

impl SnapshotEntry {
    /// Create a new snapshot entry
    fn new(path: &Path, file_type: &fs::FileType, metadata: Option<fs::Metadata>) -> Self {
        let metadata = metadata.map(|metadata| {
            #[cfg(target_family = "unix")]
            let (uid, gid) = {
                use std::os::unix::prelude::MetadataExt;
                (metadata.uid(), metadata.gid())
            };

            #[cfg(not(target_family = "unix"))]
            let (uid, gid) = (1000u32, 1000u32);

            SnapshotEntryMetadata {
                uid,
                gid,
                readonly: metadata.permissions().readonly(),
            }
        });

        let fingerprint = if file_type.is_file() {
            match Self::file_fingerprint::<SeaHasher>(path) {
                Ok(fingerprint) => Some(fingerprint),
                Err(error) => {
                    error!("While fingerprinting file `{}`: {}", path.display(), error);
                    None
                }
            }
        } else {
            None
        };

        let target = if file_type.is_symlink() {
            match fs::read_link(path) {
                Ok(target) => Some(target.to_string_lossy().to_string()),
                Err(error) => {
                    error!(
                        "While reading target of symlink `{}`: {}",
                        path.display(),
                        error
                    );
                    None
                }
            }
        } else {
            None
        };

        Self {
            metadata,
            fingerprint,
            target,
        }
    }

    /// Generate a hash of a file's content
    ///
    /// Used to generate a fingerprint
    ///
    /// Based on https://github.com/jRimbault/yadf/blob/04205a57882ffa7d6a9ca05016e18214a38079b6/src/fs/hash.rs#L29
    fn file_fingerprint<H>(path: &Path) -> io::Result<u64>
    where
        H: Hasher + Default,
    {
        struct HashWriter<H>(H);
        impl<H: Hasher> io::Write for HashWriter<H> {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                self.0.write(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut hasher = HashWriter(H::default());
        io::copy(&mut std::fs::File::open(path)?, &mut hasher)?;
        Ok(hasher.0.finish())
    }
}

impl ChangeSet {
    /// Create a new set of snapshot changes
    pub fn new<P: AsRef<Path>>(source_dir: P, dest_dir: P, items: Vec<Change>) -> Self {
        let source_dir = source_dir.as_ref().to_path_buf();

        // Parths in tar archive must be relative so stri any leading slash
        let dest_dir = dest_dir.as_ref().to_path_buf();
        let dest_dir = match dest_dir.strip_prefix("/") {
            Ok(dir) => dir.to_owned(),
            Err(_) => dest_dir,
        };

        Self {
            source_dir,
            dest_dir,
            items,
        }
    }

    /// Creates an OCI layer for the set of changes
    ///
    /// This implements the [Representing Changes](https://github.com/opencontainers/image-spec/blob/main/layer.md#representing-changes)
    /// section of the OCI image spec:
    ///
    /// - `Added` and `Modified` paths are added to the archive.
    /// - `Removed` paths are represented as "whiteout" files.
    ///
    /// Note that two SHA256 hashes are calculated, one for the `DiffID` of a changeset (calculated in this function
    /// and used in the image config file) and one for the digest which (calculated by the [`BlobWriter`] and used in the image manifest).
    /// A useful diagram showing how these are calculated and used is available
    /// [here](https://github.com/google/go-containerregistry/blob/main/pkg/v1/remote/README.md#anatomy-of-an-image-upload).
    ///
    /// # Arguments
    ///
    /// - `layout_dir`: the image directory to write the layer to (to the `blob/sha256` subdirectory)
    pub fn write_layer<P: AsRef<Path>>(
        mut self,
        layout_dir: P,
        media_type: &MediaType,
    ) -> Result<(String, Descriptor)> {
        if self.items.is_empty() {
            return Ok((
                "<empty>".to_string(),
                DescriptorBuilder::default()
                    .media_type(media_type.clone())
                    .digest("<none>")
                    .size(0)
                    .build()?,
            ));
        }

        log::info!(
            "Writing image layer from changeset for `{}`",
            self.source_dir.display()
        );

        let mut diffid_hash = Sha256::new();
        let mut blob_writer = BlobWriter::new(&layout_dir, media_type.to_owned())?;

        let changes = self.items.len();
        let mut additions: Vec<String> = Vec::new();
        let mut modifications: Vec<String> = Vec::new();
        let mut deletions: Vec<String> = Vec::new();

        {
            enum LayerEncoder<'lt> {
                Plain(&'lt mut BlobWriter),
                // Gzip(flate2::write::GzEncoder<&'lt mut BlobWriter>),
                // Zstd(zstd::stream::AutoFinishEncoder<'lt, &'lt mut BlobWriter>),
            }

            struct LayerWriter<'lt> {
                diffid_hash: &'lt mut Sha256,
                layer_encoder: LayerEncoder<'lt>,
            }

            impl<'lt> io::Write for LayerWriter<'lt> {
                fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                    // debug!("write {}: {:?}", buf.len(), buf);
                    self.diffid_hash.update(buf);
                    match &mut self.layer_encoder {
                        LayerEncoder::Plain(encoder) => encoder.write_all(buf)?,
                        // LayerEncoder::Gzip(encoder) => encoder.write_all(buf)?,
                        // LayerEncoder::Zstd(encoder) => encoder.write_all(buf)?,
                    }
                    Ok(buf.len())
                }

                fn flush(&mut self) -> io::Result<()> {
                    Ok(())
                }
            }

            let layer_encoder = match media_type {
                MediaType::ImageLayer => LayerEncoder::Plain(&mut blob_writer),
                // MediaType::ImageLayerGzip => LayerEncoder::Gzip(flate2::write::GzEncoder::new(
                //     &mut blob_writer,
                //     flate2::Compression::new(4),
                // )),
                // MediaType::ImageLayerZstd => LayerEncoder::Zstd(
                //     zstd::stream::Encoder::new(&mut blob_writer, 4)?.auto_finish(),
                // ),
                _ => bail!("Unhandled media type for layer: {}", media_type.to_string()),
            };

            let mut layer_writer = LayerWriter {
                diffid_hash: &mut diffid_hash,
                layer_encoder,
            };

            let mut archive = tar::Builder::new(&mut layer_writer);

            // Add an entry for the `dest_dir` (and any of its parent) so that ownership (and other
            // metadata) of `source_dir` is maintained. If not done then there are issues with non-root
            // users writing to the `workspace` and `layers` directories and  their subdirectories.
            let mut path = PathBuf::new();
            for part in self.dest_dir.components() {
                path = path.join(part);
                archive.append_path_with_name(&self.source_dir, &path)?;
            }

            self.items.sort();
            debug!("changes: {:?}", self.items);
            // Add each change
            for change in self.items {
                match change {
                    Change::Added(ref path) | Change::Modified(ref path) => {
                        let source_path = self.source_dir.join(path);
                        let dest_path = self.dest_dir.join(path);

                        let result = if source_path.is_symlink() {
                            match fs::read_link(&source_path).and_then(|target| {
                                fs::metadata(&source_path).map(|metadata| (target, metadata))
                            }) {
                                Ok((target, _metadata)) => {
                                    #[cfg(target_family = "unix")]
                                    let (uid, gid) = {
                                        use std::os::unix::prelude::MetadataExt;
                                        (_metadata.uid(), _metadata.gid())
                                    };

                                    #[cfg(not(target_family = "unix"))]
                                    let (uid, gid) = (1000u32, 1000u32);

                                    let mut header = tar::Header::new_gnu();
                                    header.set_uid(uid.into());
                                    header.set_gid(gid.into());
                                    header.set_entry_type(tar::EntryType::Symlink);
                                    header.set_size(0);
                                    archive.append_link(&mut header, dest_path, target)
                                }
                                Err(error) => Err(error),
                            }
                        } else {
                            debug!("source_path={:?} dest_path={:?}", source_path, dest_path);

                            archive.append_path_with_name(source_path, dest_path)
                        };

                        if let Err(error) = result {
                            debug!(
                                "While appending item for added or modified path `{}`: {}",
                                path, error
                            )
                        } else {
                            match change {
                                Change::Added(..) => additions.push(path.to_string()),
                                Change::Modified(..) => modifications.push(path.to_string()),
                                _ => unreachable!(),
                            }
                        }
                    }
                    Change::Removed(path) => {
                        let path_buf = PathBuf::from(&path);
                        let basename = path_buf
                            .file_name()
                            .ok_or_else(|| anyhow!("Path has no file name"))?;
                        let mut whiteout = OsString::from(".wh.".to_string());
                        whiteout.push(basename);
                        let path_buf = match path_buf.parent() {
                            Some(parent) => parent.join(whiteout),
                            None => PathBuf::from(whiteout),
                        };
                        let path_buf = self.dest_dir.join(path_buf);

                        let mut header = tar::Header::new_gnu();
                        header.set_path(path_buf)?;
                        header.set_size(0);
                        header.set_cksum();
                        let data: &[u8] = &[];

                        if let Err(error) = archive.append(&header, data) {
                            debug!(
                                "While appending item for deleted path `{}`: {}",
                                path, error
                            )
                        } else {
                            deletions.push(path)
                        }
                    }
                };
            }
        }
        let diff_id = format!("sha256:{:x}", diffid_hash.finalize());
        debug!("layer's digest: {}", diff_id);

        let mut annotations: HashMap<String, String> = [
            ("io.stencila.version", env!("CARGO_PKG_VERSION").to_string()),
            ("io.stencila.layer.created", Utc::now().to_rfc3339()),
            (
                "io.stencila.layer.directory",
                self.dest_dir.to_string_lossy().to_string(),
            ),
            ("io.stencila.layer.changes", changes.to_string()),
        ]
        .map(|(name, value)| (name.to_string(), value))
        .into();

        fn first_100(vec: Vec<String>) -> String {
            vec[..(std::cmp::min(vec.len(), 100))].join(":")
        }
        if !additions.is_empty() {
            annotations.insert(
                "io.stencila.layer.additions".to_string(),
                first_100(additions),
            );
        }
        if !modifications.is_empty() {
            annotations.insert(
                "io.stencila.layer.modifications".to_string(),
                first_100(modifications),
            );
        }
        if !deletions.is_empty() {
            annotations.insert(
                "io.stencila.layer.deletions".to_string(),
                first_100(deletions),
            );
        }

        let descriptor = blob_writer.finish(Some(annotations))?;

        Ok((diff_id, descriptor))
    }
}
#[cfg(test)]
mod test {
    use std::path::PathBuf;

    #[test]
    fn test_is_regex() {
        let valid_ident = regex::Regex::new(r"(.*)/([^/]*)$").unwrap();
        {
            let res = valid_ident.captures("/").unwrap();
            assert_eq!(res.get(1).unwrap().as_str(), "");
            assert_eq!(res.get(2).unwrap().as_str(), "");
        }
        {
            let res = valid_ident.captures("/abc/").unwrap();
            assert_eq!(res.get(1).unwrap().as_str(), "/abc");
            assert_eq!(res.get(2).unwrap().as_str(), "");
        }
        {
            let res = valid_ident.captures("/abc.txt").unwrap();
            assert_eq!(res.get(1).unwrap().as_str(), "");
            assert_eq!(res.get(2).unwrap().as_str(), "abc.txt");
        }
        {
            let res = valid_ident.captures("/config/abc.txt").unwrap();
            assert_eq!(res.get(1).unwrap().as_str(), "/config");
            assert_eq!(res.get(2).unwrap().as_str(), "abc.txt");
        }
        {
            assert!(valid_ident.captures("abc").is_none());
        }
        {
            let res = valid_ident.captures("config/abc.txt").unwrap();
            assert_eq!(res.get(1).unwrap().as_str(), "config");
            assert_eq!(res.get(2).unwrap().as_str(), "abc.txt");
        }
    }
    #[test]
    fn test_is_dir() {
        let path: PathBuf = "C:\\Users\\DELL\\AppData".into();
        println!("{:?}", path.is_dir());
        println!("{:?}", path.join("Local/Temp/").is_dir());
        let path: PathBuf = "C:\\Users\\DELL\\AppData\\".into();
        println!("{:?}", path.is_dir());
        println!("{:?}", path.join("Local/Temp/").is_dir());
        println!("{:?}", path.join("/Local/Temp/").is_dir());
    }
}
