
use crate::io::*;
use std::marker::PhantomData;
use std::cell::RefCell;

pub enum End<SizeType> {
    Offset(u64),
    Size(SizeType),
    None
}

/// A count of object.
/// All count object can be stored in a u32.
#[derive(PartialEq, PartialOrd)]
pub struct Count(pub u32);

pub trait Producable {
    fn produce(producer: &mut dyn Producer) -> Result<Self> where Self:Sized;
}

pub trait Indexable<T> {
    fn at(&self, idx: u32) -> Result<T>;
}

pub struct ArrayProducer<'a, T> {
    producer: RefCell<Box<dyn Producer + 'a>>,
    length: Count,
    elem_size: usize, // We know that array can contain elem of 256 at maximum.
    produced_type: PhantomData<*const T>
}

impl<'a, T: Producable> ArrayProducer<'a, T> {
    pub fn new(producer: Box<dyn Producer + 'a>, length: Count, elem_size: usize) -> Self {
        Self {
            producer: RefCell::new(producer),
            length,
            elem_size,
            produced_type: PhantomData
        }
    }

    pub fn at(&self, idx: Count) -> Result<T> {
        assert!(idx<self.length);
        let offset = (idx.0 as usize * self.elem_size) as u64;
        self.producer.borrow_mut().set_cursor(offset);
        T::produce(self.producer.borrow_mut().as_mut())
    }
}

impl<'a> ArrayProducer<'a, u64> {
    pub fn new(producer: Box<dyn Producer + 'a>, length: Count) -> Self {
        Self {
            producer: RefCell::new(producer),
            length,
            elem_size:8,
            produced_type: PhantomData
        }
    }

    pub fn at(&self, idx: Count) -> Result<u64> {
        let offset = (idx.0 as usize * self.elem_size) as u64;
        self.producer.borrow_mut().set_cursor(offset);
        self.producer.borrow_mut().read_u64()
    }
}
