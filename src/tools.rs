use crate as jbk;
use jbk::bases::*;
use jbk::reader::ContainerPack;
use std::path::Path;

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
