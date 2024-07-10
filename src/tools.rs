use crate as jbk;
use crate::common::{PackHeader, PackKind};
use crate::reader::{ManifestPackHeader, PackOffsetsIter};
use jbk::bases::*;
use jbk::reader::ContainerPack;
use std::io::{Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

pub fn concat<P: AsRef<Path>>(infiles: &[P], outfile: P) -> jbk::Result<()> {
    let mut container = jbk::creator::ContainerPackCreator::new(&outfile, Default::default())?;

    for infile in infiles {
        let in_container = open_pack(infile)?;
        for (uuid, reader) in in_container.iter() {
            container.add_pack(
                *uuid,
                &mut reader.create_stream(Offset::zero(), reader.size()),
            )?;
        }
    }

    container.finalize()?;
    Ok(())
}

pub fn open_pack<P: AsRef<Path>>(path: P) -> jbk::Result<ContainerPack> {
    let reader = Reader::from(FileSource::open(&path)?);
    let pack_header = reader.parse_block_at::<jbk::common::PackHeader>(Offset::zero())?;
    Ok(match pack_header.magic {
        jbk::common::PackKind::Container => ContainerPack::new(reader)?,
        _ => ContainerPack::new_fake(reader, pack_header.uuid),
    })
}

pub fn set_location<P: AsRef<Path>>(
    filename: P,
    uuid: Uuid,
    new_location: Vec<u8>,
) -> jbk::Result<(PackKind, Vec<u8>)> {
    let container = Arc::new(jbk::tools::open_pack(&filename)?);

    let manifest_pack_reader = container.get_manifest_pack_reader()?;
    if manifest_pack_reader.is_none() {
        return Err(format!("No manifest pack in {}", filename.as_ref().display()).into());
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

        return Ok((pack_info.pack_kind, old_location));
    }
    return Err(format!("Cannot find pack {uuid}").into());
}
