use crate::bases::*;

pub enum BlockCheck {
    None,
    Crc32,
}

impl BlockCheck {
    pub(crate) const fn size(&self) -> Size {
        match self {
            Self::None => Size::zero(),
            Self::Crc32 => Size::new(4),
        }
    }
}

pub trait BlockParsable: Parsable {}

pub trait SizedBlockParsable: BlockParsable + SizedParsable {
    const BLOCK_SIZE: usize;
}

impl<T: BlockParsable + SizedParsable> SizedBlockParsable for T {
    const BLOCK_SIZE: usize = <T as SizedParsable>::SIZE + BlockCheck::Crc32.size().into_usize();
}

pub(crate) trait DataBlockParsable {
    type TailParser: BlockParsable;
    type Output;

    fn finalize(
        intermediate: <Self::TailParser as Parsable>::Output,
        header_offset: Offset,
        reader: &Reader,
    ) -> Result<Self::Output>;
}
