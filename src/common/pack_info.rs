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
