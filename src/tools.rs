use crate as jbk;
use crate::common::{FullPackKind, PackHeader};
use jbk::bases::*;
use jbk::reader::ContainerPack;
use std::path::Path;

pub fn concat<P: AsRef<Path>>(infiles: &[P], outfile: P) -> jbk::Result<()> {
    let mut container = jbk::creator::ContainerPackCreator::new(&outfile)?;

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
    let kind = reader.parse_at::<FullPackKind>(Offset::zero())?;
    Ok(match kind {
        jbk::common::PackKind::Container => ContainerPack::new(reader)?,
        _ => {
            let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero())?;
            ContainerPack::new_fake(reader, pack_header.uuid)
        }
    })
}
