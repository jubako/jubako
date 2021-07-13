pub mod producing;
pub mod reader;
pub mod types;

use producing::*;
use reader::*;
use std::marker::PhantomData;
use types::*;

pub struct ArrayProducer<'a, T, I> {
    reader: Box<dyn Reader + 'a>,
    length: Count<I>,
    elem_size: usize, // We know that array can contain elem of 256 at maximum.
    produced_type: PhantomData<*const T>,
}

impl<'a, T: Producable, I> ArrayProducer<'a, T, I> {
    pub fn new(reader: Box<dyn Reader + 'a>, length: Count<I>, elem_size: usize) -> Self {
        Self {
            reader,
            length,
            elem_size,
            produced_type: PhantomData,
        }
    }
}

#[macro_export]
macro_rules! produceArray(
    ($reader:ident, at:$offset:expr, len:$len:expr, idx:$IDX:ty => ($OUT:ty, $elem_size:expr)) => {
        {
        let sub_reader = $reader.create_sub_reader(
            $offset,
            End::Size(Size::from(u64::from($len.0) * $elem_size))
        );
        ArrayProducer::<$OUT, $IDX>::new(
            sub_reader,
            $len,
            $elem_size)
    }}
);

impl<T: Producable, I> Index<Idx<I>> for ArrayProducer<'_, T, I>
where
    u64: std::convert::From<I>,
    I: std::cmp::PartialOrd + Copy,
{
    type OutputType = T;
    fn index(&self, idx: Idx<I>) -> T {
        assert!(idx.is_valid(self.length));
        let offset = u64::from(idx.0) * self.elem_size as u64;
        let mut producer = self
            .reader
            .create_stream(Offset::from(offset), End::Size(Size::from(self.elem_size)));
        T::produce(producer.as_mut()).unwrap()
    }
}
