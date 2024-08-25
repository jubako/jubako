use super::ValueStoreTrait;
use crate::bases::*;
use crate::common::{ContentAddress, Value};
use std::cmp;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) struct Extend {
    store: Arc<dyn ValueStoreTrait>,
    pub(crate) value_id: ValueIdx,
}

impl Extend {
    pub fn new(store: Arc<dyn ValueStoreTrait>, value_id: ValueIdx) -> Self {
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
    pub(crate) size: Option<ASize>,
    pub(crate) base: BaseArray,
    pub(crate) base_len: u8,
    pub(crate) extend: Option<Extend>,
}

impl Array {
    pub(crate) fn new(
        size: Option<ASize>,
        base: BaseArray,
        mut base_len: u8,
        extend: Option<Extend>,
    ) -> Self {
        // While the property can have a fixed base array of base_len, the actual size of the array may be shorter.
        base_len = if let Some(s) = size {
            std::cmp::min(s.into_usize(), base_len as usize) as u8
        } else {
            base_len
        };
        Self {
            size,
            base,
            base_len,
            extend,
        }
    }

    pub fn resolve_to_vec(&self, vec: &mut Vec<u8>) -> Result<()> {
        let our_iter = ArrayIter::new(self)?;
        if let Some(s) = self.size {
            vec.reserve(s.into_u64() as usize);
        } else {
            vec.reserve(self.base_len as usize);
        }
        for v in our_iter {
            vec.push(v?);
        }
        Ok(())
    }

    pub fn partial_cmp(&self, other: &[u8]) -> Result<Option<cmp::Ordering>> {
        let our_iter = ArrayIter::new(self)?;
        let mut other_iter = other.iter();
        for our_value in our_iter {
            let our_value = our_value?;
            let other_value = other_iter.next();
            match other_value {
                None => return Ok(Some(cmp::Ordering::Greater)),
                Some(other_value) => {
                    let cmp = our_value.cmp(other_value);
                    if cmp != cmp::Ordering::Equal {
                        return Ok(Some(cmp));
                    };
                }
            }
        }
        Ok(Some(match other_iter.next() {
            None => cmp::Ordering::Equal,
            Some(_) => cmp::Ordering::Less,
        }))
    }

    pub fn size(&self) -> Option<usize> {
        self.size.map(|v| v.into_usize())
    }
}

#[derive(Debug)]
enum ArrayIterMode<'a> {
    Base { data: &'a [u8], len: usize },
    Extend { data: &'a [u8], len: usize },
    End,
}

#[derive(Debug)]
struct ArrayIter<'a> {
    array: &'a Array,
    mode: ArrayIterMode<'a>,
    idx: usize,
}

impl<'a> ArrayIter<'a> {
    fn new(array: &'a Array) -> Result<Self> {
        let mode = if array.base_len > 0 {
            ArrayIterMode::Base {
                data: array.base.data.as_slice(),
                len: array.base_len as usize,
            }
        } else {
            Self::setup_extend(array)?
        };
        Ok(Self {
            array,
            mode,
            idx: 0,
        })
    }

    fn setup_extend(array: &Array) -> Result<ArrayIterMode> {
        // We may use unchecked_sub here as we know that base_len is min(v, base_len)
        let known_size = array.size.map(|v| v.into_usize() - array.base_len as usize);
        if let Some(0) = known_size {
            Ok(ArrayIterMode::End)
        } else {
            match &array.extend {
                Some(extend) => {
                    let data = extend
                        .store
                        .get_data(extend.value_id, known_size.map(ASize::new))?;
                    if data.is_empty() {
                        Ok(ArrayIterMode::End)
                    } else {
                        Ok(ArrayIterMode::Extend {
                            data,
                            len: data.len(),
                        })
                    }
                }
                None => Ok(ArrayIterMode::End),
            }
        }
    }
}

impl Iterator for ArrayIter<'_> {
    type Item = Result<u8>;
    fn next(&mut self) -> Option<Self::Item> {
        match &self.mode {
            ArrayIterMode::Base { data, len } => {
                let ret = data[self.idx];
                self.idx += 1;
                if self.idx == *len {
                    self.mode = match Self::setup_extend(self.array) {
                        Ok(mode) => mode,
                        Err(e) => {
                            return Some(Err(e));
                        }
                    };
                    self.idx = 0;
                }
                Some(Ok(ret))
            }
            ArrayIterMode::Extend { data, len } => {
                let ret = data[self.idx];
                self.idx += 1;
                if self.idx == *len {
                    self.mode = ArrayIterMode::End;
                }
                Some(Ok(ret))
            }
            ArrayIterMode::End => None,
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
            RawValue::U8(v) => Value::Unsigned(*v as u64),
            RawValue::U16(v) => Value::Unsigned(*v as u64),
            RawValue::U32(v) => Value::Unsigned(*v as u64),
            RawValue::U64(v) => Value::Unsigned(*v),
            RawValue::I8(v) => Value::Signed(*v as i64),
            RawValue::I16(v) => Value::Signed(*v as i64),
            RawValue::I32(v) => Value::Signed(*v as i64),
            RawValue::I64(v) => Value::Signed(*v),
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

    pub(crate) fn partial_cmp(&self, other: &Value) -> Result<Option<cmp::Ordering>> {
        match other {
            Value::Content(_) => Ok(None),
            Value::Unsigned(v) => Ok(match self {
                RawValue::U8(r) => Some((*r as u64).cmp(v)),
                RawValue::U16(r) => Some((*r as u64).cmp(v)),
                RawValue::U32(r) => Some((*r as u64).cmp(v)),
                RawValue::U64(r) => Some((*r).cmp(v)),
                _ => None,
            }),
            Value::UnsignedWord(v) => Ok(match self {
                RawValue::U8(r) => Some((*r as u64).cmp(&v.get())),
                RawValue::U16(r) => Some((*r as u64).cmp(&v.get())),
                RawValue::U32(r) => Some((*r as u64).cmp(&v.get())),
                RawValue::U64(r) => Some((*r).cmp(&v.get())),
                _ => None,
            }),
            Value::Signed(v) => Ok(match self {
                RawValue::I8(r) => Some((*r as i64).cmp(v)),
                RawValue::I16(r) => Some((*r as i64).cmp(v)),
                RawValue::I32(r) => Some((*r as i64).cmp(v)),
                RawValue::I64(r) => Some((*r).cmp(v)),
                _ => None,
            }),
            Value::SignedWord(v) => Ok(match self {
                RawValue::I8(r) => Some((*r as i64).cmp(&v.get())),
                RawValue::I16(r) => Some((*r as i64).cmp(&v.get())),
                RawValue::I32(r) => Some((*r as i64).cmp(&v.get())),
                RawValue::I64(r) => Some((*r).cmp(&v.get())),
                _ => None,
            }),
            Value::Array(v) => match self {
                RawValue::Array(a) => a.partial_cmp(v),
                _ => Ok(None),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::directory_pack::ValueStoreTrait;
    use crate::ContentAddress;
    use galvanic_test::test_suite;

    test_suite! {
        use super::*;

        mod mock {
            use super::*;
            #[derive(Debug)]
            pub struct ValueStore {}
            impl ValueStoreTrait for ValueStore {
                fn get_data(&self, id: ValueIdx, size: Option<ASize>) -> Result<&[u8]> {
                    let id = id.into_u64() as usize;
                    assert!(size.is_some());
                    Ok(&b"HelloWorldJubakoisawsome"[id..id+size.unwrap().into_usize()])
                }
            }
        }

        fixture value(value: RawValue, expected: Value) -> () {
            params {
                vec![
                    (RawValue::U8(5),    Value::Unsigned(5)),
                    (RawValue::U16(300), Value::Unsigned(300)),
                    (RawValue::U32(5),   Value::Unsigned(5)),
                    (RawValue::U64(5),   Value::Unsigned(5)),
                    (RawValue::I8(5),    Value::Signed(5)),
                    (RawValue::I16(5),   Value::Signed(5)),
                    (RawValue::I32(5),   Value::Signed(5)),
                    (RawValue::I64(5),   Value::Signed(5)),
                    (RawValue::Array(Array{
                       size: Some(ASize::new(10)),
                       base: BaseArray::new(b"Bye "),
                       base_len: 4,
                       extend:Some(Extend{store:Arc::new(mock::ValueStore{}), value_id:ValueIdx::from(10)})
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
                    ("Hello".into(), Some(Extend{store:Arc::new(mock::ValueStore{}), value_id:ValueIdx::from(0)}), "HelloHello".into()),
                    ("Hello ".into(), Some(Extend{store:Arc::new(mock::ValueStore{}), value_id:ValueIdx::from(10)}), "Hello Jubako".into()),
                    (vec![], Some(Extend{store:Arc::new(mock::ValueStore{}), value_id:ValueIdx::from(18)}), "awsome".into()),
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
                size: Some(12.into()),
                base: BaseArray::new(b"Hello "),
                base_len: 6,
                extend: Some(Extend{store:Arc::new(mock::ValueStore{}), value_id:ValueIdx::from(10)})
            };
            assert_eq!(raw_value.partial_cmp(&"Hel".as_bytes()).unwrap().unwrap(), cmp::Ordering::Greater);
            assert_eq!(raw_value.partial_cmp(&"Hello".as_bytes()).unwrap().unwrap(), cmp::Ordering::Greater);
            assert_eq!(raw_value.partial_cmp(&"Hello ".as_bytes()).unwrap().unwrap(), cmp::Ordering::Greater);
            assert_eq!(raw_value.partial_cmp(&"Hello Jubako".as_bytes()).unwrap().unwrap(), cmp::Ordering::Equal);
            assert_eq!(raw_value.partial_cmp(&"Hello Jubako!".as_bytes()).unwrap().unwrap(), cmp::Ordering::Less);
            assert_eq!(raw_value.partial_cmp(&"Hella Jubako!".as_bytes()).unwrap().unwrap(), cmp::Ordering::Greater);
            assert_eq!(raw_value.partial_cmp(&"Hemmo Jubako!".as_bytes()).unwrap().unwrap(), cmp::Ordering::Less);

        }
    }
}
