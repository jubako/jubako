pub mod producing;
pub mod types;

use producing::*;
use std::cell::RefCell;
use std::io::SeekFrom;
use std::marker::PhantomData;
use types::*;

impl Producable for Offset {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        Ok(producer.read_u64()?.into())
    }
}

impl Producable for Size {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        Ok(producer.read_u64()?.into())
    }
}

impl Producable for Count<u16> {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        Ok(producer.read_u16()?.into())
    }
}

impl Producable for Count<u32> {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        Ok(producer.read_u32()?.into())
    }
}

pub struct ArrayProducer<'a, T, I> {
    producer: RefCell<Box<dyn Producer + 'a>>,
    length: Count<I>,
    elem_size: usize, // We know that array can contain elem of 256 at maximum.
    produced_type: PhantomData<*const T>,
}

impl<'a, T: Producable, I> ArrayProducer<'a, T, I> {
    pub fn new(producer: Box<dyn Producer + 'a>, length: Count<I>, elem_size: usize) -> Self {
        Self {
            producer: RefCell::new(producer),
            length,
            elem_size,
            produced_type: PhantomData,
        }
    }
}

impl<T: Producable, I> Index<Idx<I>> for ArrayProducer<'_, T, I>
where
    u64: std::convert::From<I>,
    I: std::cmp::PartialOrd + Copy,
{
    type OutputType = T;
    fn index(&self, idx: Idx<I>) -> T {
        assert!(idx.is_valid(self.length));
        let offset = u64::from(idx.0) * self.elem_size as u64;
        self.producer
            .borrow_mut()
            .seek(SeekFrom::Start(offset))
            .unwrap();
        T::produce(self.producer.borrow_mut().as_mut()).unwrap()
    }
}
