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

impl SizedParsable for PackLocator {
    const SIZE: usize = Uuid::SIZE + Size::SIZE + Offset::SIZE;
}

impl Parsable for PackLocator {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        let uuid = Uuid::parse(parser)?;
        let pack_size = Size::parse(parser)?;
        let pack_pos = Offset::parse(parser)?;
        Ok(Self {
            uuid,
            pack_size,
            pack_pos,
        })
    }
}
