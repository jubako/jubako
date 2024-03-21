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

impl PackInfo {
    pub fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        self.uuid.write(stream)?;
        self.pack_size.write(stream)?;
        self.check_info_pos.write(stream)?;
        self.pack_id.write(stream)?;
        self.pack_kind.write(stream)?;
        stream.write_u8(self.pack_group)?;
        stream.write_u16(self.free_data_id.into_u64() as u16)?;
        PString::write_string_padded(self.pack_location.as_ref(), 217, stream)?;
        Ok(())
    }
}

impl SizedProducable for PackInfo {
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

impl Producable for PackInfo {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let uuid = Uuid::produce(flux)?;
        let pack_size = Size::produce(flux)?;
        let check_info_pos = Offset::produce(flux)?;
        let pack_id = flux.read_u16()?.into();
        let pack_kind = PackKind::produce(flux)?;
        let pack_group = flux.read_u8()?;
        let free_data_id = ValueIdx::from(flux.read_u16()? as u64);
        let pack_location = PString::produce(flux)?;
        flux.skip(Size::from(217 - pack_location.len()))?;
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
