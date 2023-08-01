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

impl PackLocator {
    pub fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        self.uuid.write(stream)?;
        self.pack_size.write(stream)?;
        self.pack_pos.write(stream)?;
        Ok(())
    }
}

impl SizedProducable for PackLocator {
    type Size = typenum::U32;
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
