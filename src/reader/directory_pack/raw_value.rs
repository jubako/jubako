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

    pub fn resolve_to_vec(&self, vec: &mut SmallBytes) -> Result<()> {
        vec.reserve(
            self.size
                .map(|s| s.into_u64())
                .unwrap_or(self.base_len as u64) as usize,
        );
        // Array::new ensure us to have base_len <= self.size and
        // jubako format ensure that base_len <= self.base.data.len()
        vec.extend_from_slice(&self.base.data[..self.base_len as usize]);
        if let Some(e) = &self.extend {
            let data = e.store.get_data(
                e.value_id,
                self.size.map(|s| s - ASize::new(self.base_len as usize)),
            )?;
            vec.extend_from_slice(data)
        };

        Ok(())
    }

    pub fn cmp(&self, other: &[u8]) -> Result<cmp::Ordering> {
        let our_iter = ArrayIter::new(self)?;
        let mut other_iter = other.iter();
        for our_value in our_iter {
            let our_value = our_value?;
            let other_value = other_iter.next();
            match other_value {
                None => return Ok(cmp::Ordering::Greater),
                Some(other_value) => {
                    let cmp = our_value.cmp(other_value);
                    if cmp != cmp::Ordering::Equal {
                        return Ok(cmp);
                    };
                }
            }
        }
        Ok(match other_iter.next() {
            None => cmp::Ordering::Equal,
            Some(_) => cmp::Ordering::Less,
        })
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

    fn setup_extend(array: &Array) -> Result<ArrayIterMode<'_>> {
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
                let mut vec = SmallBytes::new();
                a.resolve_to_vec(&mut vec)?;
                Value::Array(vec)
            }
        })
    }

    pub fn as_vec(&self) -> Result<SmallBytes> {
        if let RawValue::Array(a) = self {
            let mut vec = SmallBytes::new();
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
                RawValue::Array(a) => Ok(Some(a.cmp(v)?)),
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

    mod mock {
        use super::*;
        #[derive(Debug)]
        pub struct ValueStore {}
        impl ValueStoreTrait for ValueStore {
            fn get_data(&self, id: ValueIdx, size: Option<ASize>) -> Result<&[u8]> {
                let id = id.into_u64() as usize;
                assert!(size.is_some());
                Ok(&b"HelloWorldJubakoisawsome"[id..id + size.unwrap().into_usize()])
            }
        }
    }

    #[derive(Clone)]
    pub struct ResolveTestCase(&'static str, RawValue, Value);

    impl rustest::ParamName for ResolveTestCase {
        fn param_name(&self) -> String {
            self.0.to_string()
        }
    }

    #[rustest::test(params:ResolveTestCase=[
            ResolveTestCase("u8(5)",RawValue::U8(5),    Value::Unsigned(5)),
            ResolveTestCase("U16(300)", RawValue::U16(300), Value::Unsigned(300)),
            ResolveTestCase("U32(5)", RawValue::U32(5),   Value::Unsigned(5)),
            ResolveTestCase("U64(5)", RawValue::U64(5),   Value::Unsigned(5)),
            ResolveTestCase("I8(5)",RawValue::I8(5),    Value::Signed(5)),
            ResolveTestCase("I16(5)", RawValue::I16(5),   Value::Signed(5)),
            ResolveTestCase("I32(5)", RawValue::I32(5),   Value::Signed(5)),
            ResolveTestCase("I64(5)", RawValue::I64(5),   Value::Signed(5)),
            ResolveTestCase("Array(Bye Jubako)", RawValue::Array(Array{
               size: Some(ASize::new(10)),
               base: BaseArray::new(b"Bye "),
               base_len: 4,
               extend:Some(Extend{store:Arc::new(mock::ValueStore{}), value_id:ValueIdx::from(10)})
             }),
             Value::Array("Bye Jubako".into())),
            ResolveTestCase("ContentAddress", RawValue::Content(ContentAddress::new(PackId::from(0), ContentIdx::from(50))),
               Value::Content(ContentAddress::new(PackId::from(0), ContentIdx::from(50)))),
        ])]
    fn test_resolver_resolve(Param(ResolveTestCase(_, value, expected)): Param) {
        assert_eq!(value.get().unwrap(), expected);
    }

    #[rustest::test]
    fn test_resolver_unsigned() {
        assert_eq!(RawValue::U8(0).as_unsigned(), 0);
        assert_eq!(RawValue::U8(5).as_unsigned(), 5);
        assert_eq!(RawValue::U8(255).as_unsigned(), 255);
        assert_eq!(RawValue::U16(300).as_unsigned(), 300);
        assert_eq!(RawValue::U32(30000).as_unsigned(), 30000);
        assert_eq!(RawValue::U64(300000000).as_unsigned(), 300000000);
    }

    #[rustest::test]
    fn test_resolver_signed() {
        assert_eq!(RawValue::I8(0).as_signed(), 0);
        assert_eq!(RawValue::I8(5).as_signed(), 5);
        assert_eq!(RawValue::I8(-1).as_signed(), -1);
        assert_eq!(RawValue::I16(300).as_signed(), 300);
        assert_eq!(RawValue::I16(-300).as_signed(), -300);
        assert_eq!(RawValue::I32(30000).as_signed(), 30000);
        assert_eq!(RawValue::I32(-30000).as_signed(), -30000);
        assert_eq!(RawValue::I64(300000000).as_signed(), 300000000);
        assert_eq!(RawValue::I64(-300000000).as_signed(), -300000000);
    }

    #[derive(Clone)]
    pub struct IndirectTestCase(&'static [u8], Option<Extend>, SmallBytes);

    impl rustest::ParamName for IndirectTestCase {
        fn param_name(&self) -> String {
            self.0.param_name()
        }
    }

    #[rustest::test(params:pub(crate)IndirectTestCase=[
            IndirectTestCase(b"", None, SmallBytes::new()),
            IndirectTestCase(b"Hello", None, "Hello".into()),
            IndirectTestCase(b"Hello", Some(Extend{store:Arc::new(mock::ValueStore{}), value_id:ValueIdx::from(0)}), "HelloHello".into()),
            IndirectTestCase(b"Hello ", Some(Extend{store:Arc::new(mock::ValueStore{}), value_id:ValueIdx::from(10)}), "Hello Jubako".into()),
            IndirectTestCase(b"", Some(Extend{store:Arc::new(mock::ValueStore{}), value_id:ValueIdx::from(18)}), "awsome".into()),
        ])]
    fn test_resolver_indirect(Param(IndirectTestCase(base, extend, expected)): Param) {
        let raw_value = RawValue::Array(Array {
            size: Some(expected.len().into()),
            base: BaseArray::new(base),
            base_len: base.len() as u8,
            extend: extend.clone(),
        });
        assert_eq!(raw_value.as_vec().unwrap(), expected)
    }

    #[rustest::test]
    fn test_resolver_compare() {
        let raw_value = Array {
            size: Some(12.into()),
            base: BaseArray::new(b"Hello "),
            base_len: 6,
            extend: Some(Extend {
                store: Arc::new(mock::ValueStore {}),
                value_id: ValueIdx::from(10),
            }),
        };
        assert_eq!(
            raw_value.cmp("Hel".as_bytes()).unwrap(),
            cmp::Ordering::Greater
        );
        assert_eq!(
            raw_value.cmp("Hello".as_bytes()).unwrap(),
            cmp::Ordering::Greater
        );
        assert_eq!(
            raw_value.cmp("Hello ".as_bytes()).unwrap(),
            cmp::Ordering::Greater
        );
        assert_eq!(
            raw_value.cmp("Hello Jubako".as_bytes()).unwrap(),
            cmp::Ordering::Equal
        );
        assert_eq!(
            raw_value.cmp("Hello Jubako!".as_bytes()).unwrap(),
            cmp::Ordering::Less
        );
        assert_eq!(
            raw_value.cmp("Hella Jubako!".as_bytes()).unwrap(),
            cmp::Ordering::Greater
        );
        assert_eq!(
            raw_value.cmp("Hemmo Jubako!".as_bytes()).unwrap(),
            cmp::Ordering::Less
        );
    }
}
