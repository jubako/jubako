use crate::bases::*;
use uuid::Uuid;

#[derive(PartialEq, Eq, Debug)]
pub struct PackLocator {
    pub uuid: Uuid,
    pub pack_size: Size,
    pub pack_pos: Offset,
}

impl PackLocator {
    pub fn new(uuid: Uuid, pack_size: Size, pack_pos: Offset) -> Self {
        Self {
            uuid,
            pack_size,
            pack_pos,
        }
    }
}

impl Serializable for PackLocator {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += self.uuid.serialize(ser)?;
        written += self.pack_size.serialize(ser)?;
        written += self.pack_pos.serialize(ser)?;
        Ok(written)
    }
}

impl SizedProducable for PackLocator {
    const SIZE: usize = Uuid::SIZE + Size::SIZE + Offset::SIZE;
}

impl Producable for PackLocator {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let uuid = Uuid::produce(flux)?;
        let pack_size = Size::produce(flux)?;
        let pack_pos = Offset::produce(flux)?;
        Ok(Self {
            uuid,
            pack_size,
            pack_pos,
        })
    }
}
