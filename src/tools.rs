use crate as jbk;
use jbk::bases::*;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom};
use std::path::Path;

pub fn concat<P: AsRef<Path>>(infiles: &[P], outfile: P) -> jbk::Result<()> {
    let manifest_path = infiles.first().unwrap();
    let mut manifest_file = File::open(&manifest_path)?;

    let mut outfile = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&outfile)?;

    std::io::copy(&mut manifest_file, &mut outfile)?;
    manifest_file.seek(SeekFrom::Start(0))?;

    let mut pack_path = manifest_path.as_ref().to_path_buf();

    let reader = jbk::bases::FileReader::new(manifest_file, jbk::End::None);
    let mut stream = reader.create_stream_all();

    let manifest_header = jbk::common::ManifestPackHeader::produce(stream.as_mut())?;

    for pack_nb in manifest_header.pack_count + 1 {
        let pack_info = jbk::reader::PackInfo::produce(stream.as_mut())?;
        match pack_info.pack_pos {
            jbk::common::PackPos::Offset(_) => {} // Nothing to do, it is already in the file,
            jbk::common::PackPos::Path(p) => {
                pack_path.set_file_name(String::from_utf8(p)?);
                let mut file = File::open(&pack_path)?;
                let pos = outfile.seek(SeekFrom::End(0))?;
                std::io::copy(&mut file, &mut outfile)?;
                outfile.seek(SeekFrom::Start(128 + 256 * pack_nb.into_u64() + 128))?;
                Offset(pos).write(&mut outfile)?;
            }
        };
    }
    Ok(())
}
