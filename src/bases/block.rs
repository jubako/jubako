use crate::bases::*;

pub trait BlockParsable: Parsable {}

pub trait SizedBlockParsable: BlockParsable + SizedParsable {
    const BLOCK_SIZE: usize;
}

impl<T: BlockParsable + SizedParsable> SizedBlockParsable for T {
    const BLOCK_SIZE: usize = <T as SizedParsable>::SIZE;
}

pub(crate) trait DataBlockParsable {
    type Intermediate;
    type TailParser: BlockParsable<Output = (Self::Intermediate, Size)>;
    type Output;

    fn finalize(intermediate: Self::Intermediate, reader: SubReader) -> Result<Self::Output>;
}
