use std::sync::Arc;

use camino::{Utf8Path, Utf8PathBuf};
use pathdiff::diff_utf8_paths;

use super::{
    content_pack::{CompHint, ContentAdder},
    AtomicOutFile, Compression, ContainerPackCreator, ContentPackCreator, DirectoryPackCreator,
    InContainerFile, InputReader, ManifestPackCreator, PackRecipient, Progress,
};
use crate::{
    bases::*,
    creator::{Error, Result},
    ContentAddress,
};

/// How packs will be stored
#[derive(Clone, Copy)]
pub enum ConcatMode {
    /// All packs will be stored in one file
    OneFile,

    /// Manifest and directory packs will be stored together and content pack will
    /// be stored separatly (with a extra `.jbkc` extension).
    TwoFiles,

    /// Manifest, directory and content packs will be store separatly with respective
    /// extensions `.jbkm`, `jbkd` and `.jbkc` added.
    NoConcat,
}

/// EntryStore must finalizable
pub trait EntryStoreTrait {
    /// Finalize EntryStore creation
    ///
    /// Most custom entry store creator will wrap value and entry stores.
    /// This method must add them to `directory_pack`.
    fn finalize(self: Box<Self>, directory_pack: &mut DirectoryPackCreator);
}

/// BasicCreator provides a simplify way to create a Jubako container.
///
/// A Jubako container is composed of several packs.
/// Writing them correctly in a efficient way can be difficult.
/// BasicCreator provides a simplify API and handle most of the generic, borring part
/// of Jubako creation.
pub struct BasicCreator {
    directory_pack: DirectoryPackCreator,
    content_pack: ContentPackCreator<InContainerFile<AtomicOutFile>>,
    concat_mode: ConcatMode,
    vendor_id: VendorId,
    outpath: Utf8PathBuf,
}

fn new_with_extension(path: &Utf8Path, extension: &str) -> Utf8PathBuf {
    let mut buf = path.to_path_buf();
    buf.set_extension(extension);
    buf
}

impl BasicCreator {
    /// Create a BasicCreator.
    ///
    /// Temporary files will be stored in `tempdir`. For performance reason, it is adviced
    /// to using a tempdir in same filesystem than final files to avoid copy of file between
    /// fs at end of creation process.
    pub fn new(
        outpath: impl AsRef<Utf8Path>,
        concat_mode: ConcatMode,
        vendor_id: VendorId,
        compression: Compression,
        progress: Arc<dyn Progress>,
    ) -> Result<Self> {
        let outpath = camino::absolute_utf8(outpath.as_ref())?;
        let atomic_content_pack_file = if let ConcatMode::OneFile = concat_mode {
            AtomicOutFile::new(&outpath)?
        } else {
            AtomicOutFile::new(new_with_extension(&outpath, "jbkc"))?
        };
        // We may have only one (content) pack in the container so container may not be necessary.
        // But let's put all in a container. It is simpler and it can simplify things if
        // user want to concat packs later.
        let tmp_content_pack =
            ContainerPackCreator::from_file(atomic_content_pack_file, Default::default())?
                .into_file()?;

        let content_pack = ContentPackCreator::new_from_output_with_progress(
            tmp_content_pack,
            PackId::from(1),
            vendor_id,
            Default::default(),
            compression,
            progress,
        )?;

        let directory_pack =
            DirectoryPackCreator::new(PackId::from(0), vendor_id, Default::default());

        Ok(Self {
            directory_pack,
            content_pack,
            concat_mode,
            vendor_id,
            outpath,
        })
    }

    /// Finalize the creation of Jubako container and create the archive as `outfile`.
    pub fn finalize(
        mut self,
        entry_store_creator: Box<dyn EntryStoreTrait>,
        extra_content_pack_creators: Vec<ContentPackCreator<dyn PackRecipient>>,
    ) -> Result<()> {
        let parent_path = self
            .outpath
            .parent()
            .expect("outfile is absolute and have a parent");
        let relative_locator = |p: Utf8PathBuf| {
            if p.as_str().is_empty() {
                p
            } else {
                diff_utf8_paths(p, parent_path).expect("outfile is absolute")
            }
        };
        entry_store_creator.finalize(&mut self.directory_pack);
        let finalized_directory_pack_creator = self.directory_pack.finalize()?;

        let (content_pack_file, content_pack_info) = self.content_pack.finalize()?;
        let (mut container, content_locator) = {
            let container = content_pack_file.close(content_pack_info.uuid)?;
            match self.concat_mode {
                ConcatMode::OneFile => {
                    // Don't close the container as we will add new pack in it.
                    (Some(container), Utf8PathBuf::new())
                }
                _ => {
                    // We must close the container, persist it but, maybe create a new one.
                    let container_file = container.finalize()?;
                    let content_pack_path = container_file.close_file()?;
                    let new_container = if let ConcatMode::TwoFiles = self.concat_mode {
                        // We have to create a new container creator for other packs

                        let atomic_container_pack = AtomicOutFile::new(&self.outpath)?;
                        // We may have only one (content) pack in the container so container may not be necessary.
                        // But let's put all in a container. It is simpler and it can simplify things if
                        // user want to concat packs later.
                        let tmp_container_pack = ContainerPackCreator::from_file(
                            atomic_container_pack,
                            Default::default(),
                        )?;
                        Some(tmp_container_pack)
                    } else {
                        None
                    };
                    (new_container, content_pack_path)
                }
            }
        };

        let extra_locators = extra_content_pack_creators
            .into_iter()
            .map(|extra_creator| {
                let (extra_pack_file, extra_pack_info) = extra_creator.finalize()?;
                let extra_path = extra_pack_file.close_file()?;
                Ok::<_, Error>((extra_pack_info, extra_path))
            })
            .collect::<Result<Vec<_>>>()?;

        let (directory_pack_info, directory_locator) = match container.take() {
            Some(inner_container) => {
                // Write directory pack in container
                let mut infile = inner_container.into_file()?;
                let directory_pack_info = finalized_directory_pack_creator.write(&mut infile)?;
                container = Some(infile.close(directory_pack_info.uuid)?);
                (directory_pack_info, Utf8PathBuf::new())
            }
            None => {
                // Write directory pack in its own file
                let mut atomic_tmp_file =
                    AtomicOutFile::new(new_with_extension(&self.outpath, ".jbkd"))?;
                let directory_pack_info =
                    finalized_directory_pack_creator.write(&mut atomic_tmp_file)?;
                let directory_pack_path = atomic_tmp_file.close_file()?;
                (directory_pack_info, directory_pack_path)
            }
        };

        // Time to build our manifest
        let mut manifest_creator = ManifestPackCreator::new(self.vendor_id, Default::default());
        manifest_creator.add_pack(directory_pack_info, relative_locator(directory_locator));
        manifest_creator.add_pack(content_pack_info, relative_locator(content_locator));

        for (extra_pack_info, extra_locator) in extra_locators {
            manifest_creator.add_pack(extra_pack_info, relative_locator(extra_locator));
        }

        match container.take() {
            Some(inner_container) => {
                let mut infile = inner_container.into_file()?;
                let manifest_uuid = manifest_creator.finalize(&mut infile)?;
                container = Some(infile.close(manifest_uuid)?);
            }
            None => {
                // Write manifest in its own file
                let mut atomic_tmp_file = AtomicOutFile::new(&self.outpath)?;
                manifest_creator.finalize(&mut atomic_tmp_file)?;
                atomic_tmp_file.close_file()?;
            }
        }

        if let Some(container) = container {
            // We have a container not closed (and not persisted)
            let container_file = container.finalize()?;
            container_file.close_file()?;
        }

        Ok(())
    }

    pub fn add_content(
        &mut self,
        content: Box<dyn InputReader>,
        comp_hint: CompHint,
    ) -> std::io::Result<ContentAddress> {
        self.content_pack.add_content(content, comp_hint)
    }
}

impl ContentAdder for BasicCreator {
    fn add_content(
        &mut self,
        content: Box<dyn InputReader>,
        comp_hint: CompHint,
    ) -> std::io::Result<ContentAddress> {
        self.add_content(content, comp_hint)
    }
}
