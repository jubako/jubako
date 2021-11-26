#[macro_use]
mod types;
mod io;
pub mod primitive;
mod reader;
mod stream;

pub use io::*;
pub use reader::*;
use std::marker::PhantomData;
pub use stream::*;
use typenum::Unsigned;
pub use types::*;

pub trait SizedProducable: Producable {
    type Size;
}

/// ArrayReader is a wrapper a reader to access element stored as a array.
/// (Consecutif block of data of the same size).
pub struct ArrayReader<OutType, IdxType> {
    reader: Box<dyn Reader>,
    length: Count<IdxType>,
    elem_size: usize,
    produced_type: PhantomData<*const OutType>,
}

impl<OutType, IdxType> ArrayReader<OutType, IdxType>
where
    OutType: SizedProducable,
    OutType::Size: typenum::Unsigned,
    u64: std::convert::From<IdxType>,
    IdxType: Copy,
{
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
    }
}

impl<OutType: Producable, IdxType> IndexTrait<Idx<IdxType>> for ArrayReader<OutType, IdxType>
where
    u64: std::convert::From<IdxType>,
    IdxType: std::cmp::PartialOrd + Copy,
{
    type OutputType = OutType::Output;
    fn index(&self, idx: Idx<IdxType>) -> OutType::Output {
        assert!(idx.is_valid(self.length));
        let offset = u64::from(idx.0) * self.elem_size as u64;
        let mut stream = self
            .reader
            .create_stream(Offset::from(offset), End::Size(Size::from(self.elem_size)));
        OutType::produce(stream.as_mut()).unwrap()
    }
}
