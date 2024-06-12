use super::PackKind;
use crate::bases::*;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PackInfo {
    pub uuid: Uuid,
    pub pack_size: Size,
    pub check_info_pos: Offset,
    pub pack_id: PackId,
    pub pack_kind: PackKind,
    pub pack_group: u8,
    pub free_data_id: ValueIdx,
    pub pack_location: Vec<u8>,
}

impl PackInfo {
    pub fn new(
        pack_data: crate::creator::PackData,
        pack_group: u8,
        free_data_id: ValueIdx,
        offset: Offset,
        pack_location: Vec<u8>,
    ) -> Self {
        Self {
            uuid: pack_data.uuid,
            pack_size: pack_data.pack_size,
            check_info_pos: offset,
            pack_id: pack_data.pack_id,
            pack_kind: pack_data.pack_kind,
            pack_group,
            free_data_id,
            pack_location,
        }
    }
}

impl Serializable for PackInfo {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += self.uuid.serialize(ser)?;
        written += self.pack_size.serialize(ser)?;
        written += self.check_info_pos.serialize(ser)?;
        written += self.pack_id.serialize(ser)?;
        written += self.pack_kind.serialize(ser)?;
        written += ser.write_u8(self.pack_group)?;
        written += ser.write_u16(self.free_data_id.into_u64() as u16)?;
        written += PString::serialize_string_padded(self.pack_location.as_ref(), 217, ser)?;
        Ok(written)
    }
}

impl SizedParsable for PackInfo {
    const SIZE: usize =
        Uuid::SIZE
        + Size::SIZE
        + Offset::SIZE
        + 2 // pack_id
        + PackKind::SIZE
        + 1 // pack_group
        + 2 // free_data_id
        + 218 // pack locator
    ;
}

impl Parsable for PackInfo {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let uuid = Uuid::parse(parser)?;
        let pack_size = Size::parse(parser)?;
        let check_info_pos = Offset::parse(parser)?;
        let pack_id = parser.read_u16()?.into();
        let pack_kind = PackKind::parse(parser)?;
        let pack_group = parser.read_u8()?;
        let free_data_id = ValueIdx::from(parser.read_u16()? as u64);
        let pack_location = PString::parse(parser)?;
        parser.skip(217 - pack_location.len())?;
        Ok(Self {
            uuid,
            pack_size,
            check_info_pos,
            pack_id,
            pack_kind,
            pack_group,
            free_data_id,
            pack_location,
        })
    }
}

#[cfg(feature = "explorable")]
impl serde::Serialize for PackInfo {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut cont = serializer.serialize_struct("Pack", 8)?;
        cont.serialize_field("uuid", &self.uuid)?;
        cont.serialize_field("size", &self.pack_size)?;
        cont.serialize_field("id", &self.pack_id)?;
        cont.serialize_field("kind", &self.pack_kind)?;
        cont.serialize_field("group", &self.pack_group)?;
        cont.serialize_field("location", &String::from_utf8_lossy(&self.pack_location))?;
        cont.serialize_field("free_data_id", &self.free_data_id)?;
        cont.serialize_field("check_info_pos", &self.check_info_pos)?;
        cont.end()
    }
}
