use super::ValueStoreTrait;
use crate::bases::*;
use crate::common::{ContentAddress, Value};
use std::cmp;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct Extend {
    pub(crate) store: Rc<dyn ValueStoreTrait>,
    pub value_id: ValueIdx,
}

impl Extend {
    pub fn new(store: Rc<dyn ValueStoreTrait>, value_id: ValueIdx) -> Self {
        Self { store, value_id }
    }
}

impl PartialEq for Extend {
    fn eq(&self, other: &Extend) -> bool {
        self.value_id == other.value_id
    }
}

impl Eq for Extend {}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Array {
    pub size: Option<Size>,
    pub base: BaseArray,
    pub base_len: u8,
    pub extend: Option<Extend>,
}

impl Array {
    pub fn new(size: Option<Size>, base: BaseArray, base_len: u8, extend: Option<Extend>) -> Self {
        Self {
            size,
            base,
            base_len,
            extend,
        }
    }

    pub fn resolve_to_vec(&self, vec: &mut Vec<u8>) -> Result<()> {
        let our_iter = ArrayIter::new(self);
        if let Some(s) = self.size {
            vec.reserve(s.into_usize());
        } else {
            vec.reserve(self.base_len as usize);
        }
        for v in our_iter {
            vec.push(v?);
        }
        Ok(())
    }
}

impl PartialEq<[u8]> for Array {
    fn eq(&self, other: &[u8]) -> bool {
        if let Some(s) = self.size {
            if s.into_usize() != other.len() {
                return false;
            }
        } else if other.len() <= self.base_len as usize {
            return false;
        }
        let our_iter = ArrayIter::new(self);
        for (s, o) in our_iter.zip(other) {
            //[TODO] Properly handle unwrap here
            if s.unwrap() != *o {
                return false;
            }
        }
        true
    }
}

impl PartialOrd<[u8]> for Array {
    fn partial_cmp(&self, other: &[u8]) -> Option<cmp::Ordering> {
        let our_iter = ArrayIter::new(self);
        let mut other_iter = other.iter();
        for our_value in our_iter {
            //[TODO] Properly handle unwrap here
            let our_value = our_value.unwrap();
            let other_value = other_iter.next();
            //println!("cmp {our_value}, {other_value:?}");
            match other_value {
                None => return Some(cmp::Ordering::Greater),
                Some(other_value) => {
                    let cmp = our_value.cmp(other_value);
                    if cmp != cmp::Ordering::Equal {
                        return Some(cmp);
                    };
                }
            }
        }
        Some(match other_iter.next() {
            None => cmp::Ordering::Equal,
            Some(_) => cmp::Ordering::Less,
        })
    }
}

pub struct ArrayIter<'a> {
    array: &'a Array,
    idx: usize,
    known_size: Option<usize>,
}

impl<'a> ArrayIter<'a> {
    pub fn new(array: &'a Array) -> Self {
        let known_size = array.size.map(|v| v.into_usize());
        Self {
            array,
            idx: 0,
            known_size,
        }
    }
}

impl Iterator for ArrayIter<'_> {
    type Item = Result<u8>;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(s) = self.known_size {
            if self.idx >= s {
                return None;
            }
        }
        // As far as we know, we are under our known size, so we must return something.
        let base_len = self.array.base_len as usize;
        if self.idx < base_len {
            let ret = self.array.base.data[self.idx];
            self.idx += 1;
            Some(Ok(ret))
        } else if let Some(extend) = &self.array.extend {
            let data = extend.store.get_data(
                extend.value_id,
                self.array.size.map(|v| v - base_len.into()),
            );
            match data {
                Ok(data) => {
                    self.known_size = Some(base_len + data.len());
                    if self.idx - base_len < data.len() {
                        let ret = data[self.idx - base_len];
                        self.idx += 1;
                        Some(Ok(ret))
                    } else {
                        None
                    }
                }
                Err(e) => Some(Err(e)),
            }
        } else {
            self.known_size = Some(base_len);
            None
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RawValue {
    Content(ContentAddress),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Array(Array),
}

impl RawValue {
    pub fn get(&self) -> Result<Value> {
        Ok(match self {
            RawValue::Content(c) => Value::Content(*c),
            RawValue::U8(v) => Value::Unsigned((*v as u64).into()),
            RawValue::U16(v) => Value::Unsigned((*v as u64).into()),
            RawValue::U32(v) => Value::Unsigned((*v as u64).into()),
            RawValue::U64(v) => Value::Unsigned((*v).into()),
            RawValue::I8(v) => Value::Signed((*v as i64).into()),
            RawValue::I16(v) => Value::Signed((*v as i64).into()),
            RawValue::I32(v) => Value::Signed((*v as i64).into()),
            RawValue::I64(v) => Value::Signed((*v).into()),
            RawValue::Array(a) => {
                let mut vec = vec![];
                a.resolve_to_vec(&mut vec)?;
                Value::Array(vec)
            }
        })
    }

    pub fn as_vec(&self) -> Result<Vec<u8>> {
        if let RawValue::Array(a) = self {
            let mut vec = vec![];
            a.resolve_to_vec(&mut vec)?;
            Ok(vec)
        } else {
            panic!();
        }
    }

    pub fn as_content(&self) -> ContentAddress {
        if let RawValue::Content(c) = self {
            *c
        } else {
            panic!();
        }
    }

    pub fn as_unsigned(&self) -> u64 {
        match self {
            RawValue::U8(v) => *v as u64,
            RawValue::U16(v) => *v as u64,
            RawValue::U32(v) => *v as u64,
            RawValue::U64(v) => *v,
            _ => panic!(),
        }
    }

    pub fn as_signed(&self) -> i64 {
        match self {
            RawValue::I8(v) => *v as i64,
            RawValue::I16(v) => *v as i64,
            RawValue::I32(v) => *v as i64,
            RawValue::I64(v) => *v,
            _ => panic!(),
        }
    }
}

impl PartialEq<Value> for RawValue {
    fn eq(&self, other: &Value) -> bool {
        match other {
            Value::Content(_) => false,
            Value::Unsigned(v) => match self {
                RawValue::U8(r) => (*r as u64).eq(&v.get()),
                RawValue::U16(r) => (*r as u64).eq(&v.get()),
                RawValue::U32(r) => (*r as u64).eq(&v.get()),
                RawValue::U64(r) => (*r).eq(&v.get()),
                _ => false,
            },
            Value::Signed(v) => match self {
                RawValue::I8(r) => (*r as i64).eq(&v.get()),
                RawValue::I16(r) => (*r as i64).eq(&v.get()),
                RawValue::I32(r) => (*r as i64).eq(&v.get()),
                RawValue::I64(r) => (*r).eq(&v.get()),
                _ => false,
            },
            Value::Array(v) => match self {
                RawValue::Array(a) => a.eq(v.as_slice()),
                _ => false,
            },
        }
    }
}

impl PartialOrd<Value> for RawValue {
    fn partial_cmp(&self, other: &Value) -> Option<cmp::Ordering> {
        match other {
            Value::Content(_) => None,
            Value::Unsigned(v) => match self {
                RawValue::U8(r) => Some((*r as u64).cmp(&v.get())),
                RawValue::U16(r) => Some((*r as u64).cmp(&v.get())),
                RawValue::U32(r) => Some((*r as u64).cmp(&v.get())),
                RawValue::U64(r) => Some((*r).cmp(&v.get())),
                _ => None,
            },
            Value::Signed(v) => match self {
                RawValue::I8(r) => Some((*r as i64).cmp(&v.get())),
                RawValue::I16(r) => Some((*r as i64).cmp(&v.get())),
                RawValue::I32(r) => Some((*r as i64).cmp(&v.get())),
                RawValue::I64(r) => Some((*r).cmp(&v.get())),
                _ => None,
            },
            Value::Array(v) => match self {
                RawValue::Array(a) => a.partial_cmp(v),
                _ => None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::directory_pack::{Extend, ValueStoreTrait};
    use crate::ContentAddress;
    use galvanic_test::test_suite;
    use std::rc::Rc;

    test_suite! {
        use super::*;

        mod mock {
            use super::*;
            #[derive(Debug)]
            pub struct ValueStore {}
            impl ValueStoreTrait for ValueStore {
                fn get_data(&self, id: ValueIdx, size: Option<Size>) -> Result<&[u8]> {
                    assert!(size.is_some());
                    Ok(&b"HelloWorldJubakoisawsome"[id.into_usize()..id.into_usize()+size.unwrap().into_usize()])
                }
            }
        }

        fixture value(value: RawValue, expected: Value) -> () {
            params {
                vec![
                    (RawValue::U8(5),    Value::Unsigned(5.into())),
                    (RawValue::U16(300), Value::Unsigned(300.into())),
                    (RawValue::U32(5),   Value::Unsigned(5.into())),
                    (RawValue::U64(5),   Value::Unsigned(5.into())),
                    (RawValue::I8(5),    Value::Signed(5.into())),
                    (RawValue::I16(5),   Value::Signed(5.into())),
                    (RawValue::I32(5),   Value::Signed(5.into())),
                    (RawValue::I64(5),   Value::Signed(5.into())),
                    (RawValue::Array(Array{
                       size: Some(Size::new(10)),
                       base: BaseArray::new(b"Bye "),
                       base_len: 4,
                       extend:Some(Extend{store:Rc::new(mock::ValueStore{}), value_id:ValueIdx::from(10)})
                     }),
                     Value::Array("Bye Jubako".into())),
                    (RawValue::Content(ContentAddress::new(PackId::from(0), ContentIdx::from(50))),
                       Value::Content(ContentAddress::new(PackId::from(0), ContentIdx::from(50)))),
                ].into_iter()
            }
            setup(&mut self) {}
        }

        test test_resolver_resolve(value) {
            assert_eq!(&value.params.value.get().unwrap(), value.params.expected);
        }

        test test_resolver_unsigned() {
            assert_eq!(RawValue::U8(0).as_unsigned(), 0);
            assert_eq!(RawValue::U8(5).as_unsigned(), 5);
            assert_eq!(RawValue::U8(255).as_unsigned(), 255);
            assert_eq!(RawValue::U16(300).as_unsigned(), 300);
            assert_eq!(RawValue::U32(30000).as_unsigned(), 30000);
            assert_eq!(
                RawValue::U64(300000000).as_unsigned(),
                300000000
            );
        }

        test test_resolver_signed() {
            assert_eq!(RawValue::I8(0).as_signed(), 0);
            assert_eq!(RawValue::I8(5).as_signed(), 5);
            assert_eq!(RawValue::I8(-1).as_signed(), -1);
            assert_eq!(RawValue::I16(300).as_signed(), 300);
            assert_eq!(RawValue::I16(-300).as_signed(), -300);
            assert_eq!(RawValue::I32(30000).as_signed(), 30000);
            assert_eq!(RawValue::I32(-30000).as_signed(), -30000);
            assert_eq!(
                RawValue::I64(300000000).as_signed(),
                300000000
            );
            assert_eq!(
                RawValue::I64(-300000000).as_signed(),
                -300000000
            );
        }

        fixture indirect_value(base: Vec<u8>, extend:Option<Extend>, expected: Vec<u8>) -> RawValue {
            params {
                vec![
                    (vec![], None, vec![]),
                    ("Hello".into(), None, "Hello".into()),
                    ("Hello".into(), Some(Extend{store:Rc::new(mock::ValueStore{}), value_id:ValueIdx::from(0)}), "HelloHello".into()),
                    ("Hello ".into(), Some(Extend{store:Rc::new(mock::ValueStore{}), value_id:ValueIdx::from(10)}), "Hello Jubako".into()),
                    (vec![], Some(Extend{store:Rc::new(mock::ValueStore{}), value_id:ValueIdx::from(18)}), "awsome".into()),
                ].into_iter()
            }
            setup(&mut self) {
                RawValue::Array(Array {
                    size: Some(self.expected.len().into()),
                    base: BaseArray::new(self.base.as_slice()),
                    base_len: self.base.len() as u8,
                    extend: self.extend.clone()
                })
            }
        }

        test test_resolver_indirect(indirect_value) {
            assert_eq!(
                &indirect_value.val.as_vec().unwrap(),
                indirect_value.params.expected
            )
        }

        test test_resolver_compare() {
            let raw_value = Array {
                size: Some(Size::new(12)),
                base: BaseArray::new(b"Hello "),
                base_len: 6,
                extend: Some(Extend{store:Rc::new(mock::ValueStore{}), value_id:ValueIdx::from(10)})
            };
            assert_eq!(raw_value.partial_cmp(&"Hel".as_bytes()).unwrap(), cmp::Ordering::Greater);
            assert_eq!(raw_value.partial_cmp(&"Hello".as_bytes()).unwrap(), cmp::Ordering::Greater);
            assert_eq!(raw_value.partial_cmp(&"Hello ".as_bytes()).unwrap(), cmp::Ordering::Greater);
            assert_eq!(raw_value.partial_cmp(&"Hello Jubako".as_bytes()).unwrap(), cmp::Ordering::Equal);
            assert_eq!(raw_value.partial_cmp(&"Hello Jubako!".as_bytes()).unwrap(), cmp::Ordering::Less);
            assert_eq!(raw_value.partial_cmp(&"Hella Jubako!".as_bytes()).unwrap(), cmp::Ordering::Greater);
            assert_eq!(raw_value.partial_cmp(&"Hemmo Jubako!".as_bytes()).unwrap(), cmp::Ordering::Less);

        }
    }
}
