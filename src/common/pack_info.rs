use super::PackPos;
use crate::bases::*;
use uuid::Uuid;

#[derive(PartialEq, Eq, Debug)]
pub struct PackInfo {
    pub uuid: Uuid,
    pub pack_id: PackId,
    pub free_data: FreeData103,
    pub pack_size: Size,
    pub check_info_pos: Offset,
    pub pack_pos: PackPos,
}

impl PackInfo {
    pub fn new_at_pos(pack_data: crate::creator::PackData, offset: Offset) -> Self {
        use crate::creator::Embedded;
        let (check_info_pos, pack_pos) = match pack_data.embedded {
            Embedded::Yes =>
            // Offset is the offset of the pack
            {
                (offset + pack_data.check_info_pos, offset.into())
            }
            Embedded::No(path) =>
            // Offset is the offset of the check_info, pack_pos is the patch to the file
            {
                (offset, path.into())
            }
        };
        Self {
            uuid: pack_data.uuid,
            pack_id: pack_data.pack_id,
            free_data: pack_data.free_data,
            pack_size: pack_data.reader.size(),
            check_info_pos,
            pack_pos,
        }
    }
}

impl PackInfo {
    pub fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        self.uuid.write(stream)?;
        self.pack_id.write(stream)?;
        self.free_data.write(stream)?;
        self.pack_size.write(stream)?;
        self.check_info_pos.write(stream)?;
        match &self.pack_pos {
            PackPos::Offset(offset) => {
                offset.write(stream)?;
                PString::write_string_padded(b"", 111, stream)?;
            }
            PackPos::Path(path) => {
                stream.write_u64(0)?;
                PString::write_string_padded(path.as_ref(), 111, stream)?;
            }
        }
        Ok(())
    }
}

impl SizedProducable for PackInfo {
    type Size = typenum::U256;
}

impl Producable for PackInfo {
    type Output = Self;
    fn produce(stream: &mut Stream) -> Result<Self> {
        let uuid = Uuid::produce(stream)?;
        let pack_id = Id::produce(stream)?.into();
        let free_data = FreeData103::produce(stream)?;
        let pack_size = Size::produce(stream)?;
        let check_info_pos = Offset::produce(stream)?;
        let pack_offset = Offset::produce(stream)?;
        let pack_pos = if pack_offset.is_zero() {
            let v = PString::produce(stream)?;
            stream.skip(Size::from(111 - v.len()))?;
            PackPos::Path(v)
        } else {
            stream.skip(Size::new(112))?;
            PackPos::Offset(pack_offset)
        };
        Ok(Self {
            uuid,
            pack_id,
            free_data,
            pack_size,
            check_info_pos,
            pack_pos,
        })
    }
}
