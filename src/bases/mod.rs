#[macro_use]
mod types;
mod block;
mod cache;
#[cfg(feature = "explorable")]
mod explorable;
mod io;
mod parsing;
mod prop_type;
mod reader;
mod skip;
mod write;

pub(crate) use block::*;
pub(crate) use cache::*;
#[cfg(feature = "explorable")]
pub use explorable::Explorable;
pub use io::FileSource;
pub(crate) use io::*;
pub(crate) use parsing::*;
pub(crate) use prop_type::*;
pub(crate) use reader::CheckReader;
pub use reader::Reader;
pub(crate) use skip::*;
use std::cmp;
use std::marker::PhantomData;
pub use types::*;
pub(crate) use write::OutStream;
pub(crate) use write::*;

pub(crate) use std::io::Result as IoResult;

/// ArrayReader is a wrapper a reader to access element stored as a array.
/// (Consecutif block of data of the same size).
pub(crate) struct ArrayReader<OutType, IdxType> {
    reader: CheckReader,
    length: Count<IdxType>,
    elem_size: usize,
    produced_type: PhantomData<OutType>,
}

impl<OutType, IdxType> ArrayReader<OutType, IdxType>
where
    OutType: SizedParsable,
    u64: std::convert::From<IdxType>,
    IdxType: Copy,
{
    /*
    pub fn new_from_reader(reader: &dyn Reader, at: Offset, length: Count<IdxType>) -> Self {
        let elem_size = OutType::Size::to_u64();
        let sub_reader =
            reader.create_sub_reader(at, End::Size(Size(elem_size * u64::from(length.0))));
        Self {
            reader: sub_reader,
            length,
            elem_size: elem_size as usize,
            produced_type: PhantomData,
        }
    }*/

    pub fn new_memory_from_reader(
        reader: &Reader,
        at: Offset,
        length: Count<IdxType>,
    ) -> Result<Self> {
        let elem_size = Size::from(OutType::SIZE);
        let array_size = elem_size * length.into_base().into();
        let reader = reader.cut_check(at, array_size, BlockCheck::Crc32)?;
        Ok(Self {
            reader,
            length,
            elem_size: elem_size.into_usize(),
            produced_type: PhantomData,
        })
    }
}

impl<OutType: Parsable, IdxType> IndexTrait<Idx<IdxType>> for ArrayReader<OutType, IdxType>
where
    u64: std::convert::From<IdxType>,
    IdxType: std::cmp::PartialOrd + Copy + std::fmt::Debug,
{
    type OutputType = Result<OutType::Output>;
    fn index(&self, idx: Idx<IdxType>) -> Result<OutType::Output> {
        debug_assert!(
            idx.is_valid(self.length),
            "idx = {:?}, length = {:?}",
            idx,
            self.length
        );
        let offset = u64::from(idx.into_base()) * self.elem_size as u64;
        self.reader
            .parse_in::<OutType>(Offset::from(offset), Size::from(self.elem_size))
    }
}

pub(crate) fn needed_bytes<T>(mut val: T) -> ByteSize
where
    T: std::cmp::PartialOrd + std::ops::Shr<Output = T> + From<u8>,
{
    let mut nb_bytes = 0_usize;
    while val > 0.into() {
        val = val >> 8.into();
        nb_bytes += 1;
    }
    nb_bytes = cmp::max(nb_bytes, 1);
    nb_bytes.try_into().unwrap()
}
