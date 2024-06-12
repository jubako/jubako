use super::{Parsable, SizedParsable};

pub trait BlockParsable: Parsable {}

pub trait SizedBlockParsable: BlockParsable + SizedParsable {
    const BLOCK_SIZE: usize;
}

impl<T: BlockParsable + SizedParsable> SizedBlockParsable for T {
    const BLOCK_SIZE: usize = <T as SizedParsable>::SIZE;
}
