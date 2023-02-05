use crate as jbk;
use jbk::bases::*;
use jbk::common::Pack;
use std::fs::File;
use std::path::Path;

pub fn concat<P: AsRef<Path>>(infiles: &[P], outfile: P) -> jbk::Result<()> {
    let manifest_path = infiles.first().unwrap();

    let manifest_file = File::open(manifest_path)?;
    let reader = jbk::bases::Reader::new(FileSource::new(manifest_file), jbk::End::None);
    let mut stream = reader.create_stream_all();

    let manifest_header = jbk::common::ManifestPackHeader::produce(&mut stream)?;
    stream.seek(manifest_header.packs_offset());

    let mut creator = jbk::creator::ManifestPackCreator::new(
        &outfile,
        manifest_header.pack_header.app_vendor_id,
        manifest_header.free_data,
    );

    let mut pack_path = manifest_path.as_ref().to_path_buf();

    for _ in manifest_header.pack_count + 1 {
        let pack_info = jbk::common::PackInfo::produce(&mut stream)?;
        let pack_reader = match pack_info.pack_pos {
            jbk::common::PackPos::Offset(o) => {
                reader.create_sub_reader(o, End::Size(pack_info.pack_size))
            }
            jbk::common::PackPos::Path(p) => {
                pack_path.set_file_name(String::from_utf8(p).unwrap());
                Reader::new(
                    FileSource::new(File::open(&pack_path)?),
                    End::Size(pack_info.pack_size),
                )
            }
        };
        let pack_header = jbk::common::PackHeader::produce(&mut pack_reader.create_stream_all())?;
        let pack_data = jbk::creator::PackData {
            uuid: pack_info.uuid,
            pack_id: pack_info.pack_id,
            free_data: pack_info.free_data,
            reader: pack_reader,
            check_info_pos: pack_header.check_info_pos,
            embedded: jbk::creator::Embedded::Yes,
        };
        creator.add_pack(pack_data);
    }

    creator.finalize()?;
    Ok(())
}

pub fn open_pack<P: AsRef<Path>>(path: P) -> jbk::Result<Box<dyn Pack>> {
    let file = File::open(&path)?;
    let reader = Reader::new(FileSource::new(file), End::None);
    let pack_header = jbk::common::PackHeader::produce(&mut reader.create_stream_all())?;
    Ok(match pack_header.magic {
        jbk::common::PackKind::Manifest => Box::new(jbk::reader::ManifestPack::new(reader)?),
        jbk::common::PackKind::Directory => Box::new(jbk::reader::DirectoryPack::new(reader)?),
        jbk::common::PackKind::Content => Box::new(jbk::reader::ContentPack::new(reader)?),
    })
}
