#[macro_use]
mod types;
mod cache;
mod flux;
mod io;
mod memory_reader;
mod prop_type;
mod reader;
mod skip;
mod stream;
mod sub_reader;
mod write;

pub use cache::*;
pub use flux::*;
pub use io::*;
pub use memory_reader::*;
pub(crate) use prop_type::*;
pub use reader::*;
pub use skip::*;
use std::cmp;
use std::marker::PhantomData;
pub use stream::*;
pub use sub_reader::*;
pub use types::*;
pub use write::*;

pub type Region = Range<Offset>;

impl Region {
    #[inline]
    pub fn new_to_end(begin: Offset, end: End, size: Size) -> Self {
        let end = match end {
            End::None => size.into(),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        Self::new(begin, end)
    }

    /// Relative cut.
    /// offset and end are relative to the current region
    #[inline]
    pub fn cut_rel(&self, offset: Offset, end: End) -> Self {
        let begin = self.begin() + offset;
        let end = match end {
            End::None => self.end(),
            End::Offset(o) => self.begin() + o,
            End::Size(s) => begin + s,
        };
        debug_assert!(
            end <= self.end(),
            "end({end:?}) <= self.end({:?})",
            self.end()
        );
        Self::new(begin, end)
    }
}

pub trait SizedProducable: Producable {
    const SIZE: usize;
}

/// ArrayReader is a wrapper a reader to access element stored as a array.
/// (Consecutif block of data of the same size).
pub struct ArrayReader<OutType, IdxType> {
    reader: Reader,
    length: Count<IdxType>,
    elem_size: usize,
    produced_type: PhantomData<OutType>,
}

impl<OutType, IdxType> ArrayReader<OutType, IdxType>
where
    OutType: SizedProducable,
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
        let sub_reader =
            reader.create_sub_memory_reader(at, End::Size(elem_size * u64::from(length.0)))?;
        Ok(Self {
            reader: sub_reader,
            length,
            elem_size: elem_size.into_usize(),
            produced_type: PhantomData,
        })
    }
}

impl<OutType: Producable, IdxType> IndexTrait<Idx<IdxType>> for ArrayReader<OutType, IdxType>
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
        let offset = u64::from(idx.0) * self.elem_size as u64;
        let mut flux = self
            .reader
            .create_flux(Offset::from(offset), End::new_size(self.elem_size as u64));
        OutType::produce(&mut flux)
    }
}

pub fn needed_bytes<T>(mut val: T) -> ByteSize
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
