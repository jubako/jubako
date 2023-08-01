use super::PackKind;
use crate::bases::*;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub struct PackInfo {
    pub uuid: Uuid,
    pub pack_size: Size,
    pub check_info_pos: Offset,
    pub pack_kind: PackKind,
    pub pack_id: PackId,
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
            pack_kind: pack_data.pack_kind,
            pack_id: pack_data.pack_id,
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
        self.pack_kind.write(stream)?;
        self.pack_id.write(stream)?;
        stream.write_u8(self.pack_group)?;
        stream.write_u8(0)?; // padding
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
        + PackKind::SIZE
        + 1 // pack_id
        + 1 // pack_group
        + 1 // padding
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
        let pack_kind = PackKind::produce(flux)?;
        let pack_id = Id::produce(flux)?.into();
        let pack_group = flux.read_u8()?;
        flux.skip(Size::new(1))?;
        let free_data_id = ValueIdx::from(flux.read_u16()? as u64);
        let pack_location = PString::produce(flux)?;
        flux.skip(Size::from(217 - pack_location.len()))?;
        Ok(Self {
            uuid,
            pack_size,
            check_info_pos,
            pack_kind,
            pack_id,
            pack_group,
            free_data_id,
            pack_location,
        })
    }
}
