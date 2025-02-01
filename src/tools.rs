use crate as jbk;
use crate::common::{PackHeader, PackKind};
use crate::reader::{ManifestPackHeader, PackOffsetsIter};
use camino::Utf8Path;
use jbk::bases::*;
use jbk::reader::ContainerPack;
use std::io::{Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub fn concat(infiles: &[impl AsRef<Path>], outfile: impl AsRef<Utf8Path>) -> jbk::Result<()> {
    let mut container = jbk::creator::ContainerPackCreator::new(&outfile, Default::default())?;

    for infile in infiles {
        let in_container = open_pack(infile.as_ref())?;
        for (uuid, reader) in in_container.iter() {
            container.add_pack(
                *uuid,
                &mut reader.create_stream(Offset::zero(), reader.size(), false)?,
            )?;
        }
    }

    container.finalize()?;
    Ok(())
}

pub fn open_pack(path: impl AsRef<Path>) -> jbk::Result<ContainerPack> {
    let reader = Reader::from(FileSource::open(path.as_ref())?);
    let pack_header = reader.parse_block_at::<jbk::common::PackHeader>(Offset::zero())?;
    Ok(match pack_header.magic {
        jbk::common::PackKind::Container => ContainerPack::new(reader)?,
        _ => ContainerPack::new_fake(reader, pack_header.uuid),
    })
}

pub fn set_location(
    filename: impl AsRef<Path>,
    uuid: Uuid,
    new_location: SmallString,
) -> jbk::Result<Option<(PackKind, SmallString)>> {
    let container = Arc::new(jbk::tools::open_pack(&filename)?);

    let manifest_pack_reader = container.get_manifest_pack_reader()?;
    if manifest_pack_reader.is_none() {
        return Err(format_error!(format!(
            "No manifest pack in {}",
            filename.as_ref().display()
        )));
    };
    let manifest_pack_reader = manifest_pack_reader.unwrap();
    let pack_header = manifest_pack_reader.parse_block_at::<PackHeader>(jbk::Offset::zero())?;
    let header = manifest_pack_reader
        .parse_block_at::<ManifestPackHeader>(jbk::Offset::from(PackHeader::SIZE))?;
    let pack_offsets = PackOffsetsIter::new(pack_header.check_info_pos, header.pack_count);
    for pack_offset in pack_offsets {
        let mut pack_info =
            manifest_pack_reader.parse_block_at::<jbk::reader::PackInfo>(pack_offset)?;
        if pack_info.uuid != uuid {
            continue;
        }

        let old_location = pack_info.pack_location.to_owned();

        let global_pack_offset =
            pack_offset.into_u64() + manifest_pack_reader.global_offset().into_u64();
        pack_info.pack_location = new_location;
        // We can safely reopen the filename here as either we have opened a container or a manifest,
        // it is the same file we have actually open and read.
        // Assuming it has not being changed by OS.
        let mut file = std::fs::OpenOptions::new().write(true).open(&filename)?;
        file.seek(SeekFrom::Start(global_pack_offset))?;
        file.ser_write(&pack_info)?;

        return Ok(Some((pack_info.pack_kind, old_location)));
    }
    Ok(None)
}
