use crate::bases::*;
use zerocopy::{ByteOrder, BE};

/// Our CRC algorithm is CRC-32C (Castagnoli), without refin or refout.
/// With don't xorout to keep the property that CRC of (data + CRC) equals 0.
const CUSTOM_ALG: crc::Algorithm<u32> = crc::Algorithm {
    width: 32,
    poly: 0x1EDC6F41,
    init: 0xFFFFFFFF,
    refin: false,
    refout: false,
    xorout: 0x00000000,
    check: 0xFABBF0EA,
    residue: 0x00000000,
};

pub(crate) const CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&CUSTOM_ALG);

/// This check a "full" slice (containing data AND crc)
pub(crate) fn assert_slice_crc(buf: &[u8]) -> Result<()> {
    let data_size = buf.len() - 4;
    let slice = &buf[..data_size];
    let mut digest = CRC.digest();
    digest.update(slice);
    let checksum = digest.finalize();
    let expected_checksum = BE::read_u32(&buf[data_size..]);
    if checksum != expected_checksum {
        let found_checksum = checksum.to_be_bytes();
        return Err(format_error!(&format!(
            "Not a valid checksum : {buf:X?}. Found is {found_checksum:X?}"
        )));
    }
    Ok(())
}

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

pub(crate) trait BlockParsable: Parsable {}

pub(crate) trait SizedBlockParsable: BlockParsable + SizedParsable {
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
