use super::private::ValueStorageTrait;
use super::{Array, ArrayIter, ContentAddress, RawValue, ValueStorage};
use crate::bases::*;
use crate::common::Value;
use std::cmp;
use std::rc::Rc;

pub(crate) mod private {
    use super::*;

    pub struct Resolver<ValueStorage: ValueStorageTrait> {
        value_storage: Rc<ValueStorage>,
    }

    impl<ValueStorage: ValueStorageTrait> Clone for Resolver<ValueStorage> {
        fn clone(&self) -> Self {
            Self {
                value_storage: Rc::clone(&self.value_storage),
            }
        }
    }

    impl<ValueStorage: ValueStorageTrait> std::fmt::Debug for Resolver<ValueStorage> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            writeln!(f, "ValueStorage")
        }
    }

    impl<ValueStorage: ValueStorageTrait> Resolver<ValueStorage> {
        pub fn new(value_storage: Rc<ValueStorage>) -> Self {
            Self { value_storage }
        }

        pub fn resolve_array_to_vec(&self, array: &Array, vec: &mut Vec<u8>) -> Result<()> {
            let value_store = if let Some(e) = &array.extend {
                Some(self.value_storage.get_value_store(e.store_id)?)
            } else {
                None
            };
            let our_iter =
                ArrayIter::<'_, ValueStorage>::new(array, value_store.as_ref().map(|v| v.as_ref()));
            if let Some(s) = array.size {
                vec.reserve(s.into_usize());
            } else {
                vec.reserve(array.base_len as usize);
            }
            for v in our_iter {
                vec.push(v?);
            }
            Ok(())
        }

        pub fn resolve(&self, raw: &RawValue) -> Result<Value> {
            Ok(match raw {
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
                    self.resolve_array_to_vec(a, &mut vec)?;
                    Value::Array(vec)
                }
            })
        }

        pub fn compare_array(&self, raw: &Array, value: &[u8]) -> Result<cmp::Ordering> {
            //println!("Compare {raw:?} to {value:?}");
            let value_store = if let Some(e) = &raw.extend {
                Some(self.value_storage.get_value_store(e.store_id)?)
            } else {
                None
            };
            let our_iter =
                ArrayIter::<'_, ValueStorage>::new(raw, value_store.as_ref().map(|v| v.as_ref()));
            let mut other_iter = value.iter();
            for our_value in our_iter {
                let our_value = our_value?;
                let other_value = other_iter.next();
                //println!("cmp {our_value}, {other_value:?}");
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

        pub fn compare(&self, raw: &RawValue, value: &Value) -> Result<cmp::Ordering> {
            match value {
                Value::Content(_) => Err("Content cannot be compared.".to_string().into()),
                Value::Unsigned(v) => match raw {
                    RawValue::U8(r) => Ok((*r as u64).cmp(&v.get())),
                    RawValue::U16(r) => Ok((*r as u64).cmp(&v.get())),
                    RawValue::U32(r) => Ok((*r as u64).cmp(&v.get())),
                    RawValue::U64(r) => Ok((*r).cmp(&v.get())),
                    _ => Err("Values kind cannot be compared.".to_string().into()),
                },
                Value::Signed(v) => match raw {
                    RawValue::I8(r) => Ok((*r as i64).cmp(&v.get())),
                    RawValue::I16(r) => Ok((*r as i64).cmp(&v.get())),
                    RawValue::I32(r) => Ok((*r as i64).cmp(&v.get())),
                    RawValue::I64(r) => Ok((*r).cmp(&v.get())),
                    _ => Err("Values kind cannot be compared.".to_string().into()),
                },
                Value::Array(v) => match raw {
                    RawValue::Array(a) => self.compare_array(a, v),
                    _ => Err("Values kind cannot be compared.".to_string().into()),
                },
            }
        }

        pub fn resolve_to_vec(&self, raw: &RawValue) -> Result<Vec<u8>> {
            if let RawValue::Array(a) = raw {
                let mut vec = vec![];
                self.resolve_array_to_vec(a, &mut vec)?;
                Ok(vec)
            } else {
                panic!();
            }
        }

        pub fn resolve_to_content(&self, raw: &RawValue) -> ContentAddress {
            if let RawValue::Content(c) = raw {
                *c
            } else {
                panic!();
            }
        }

        pub fn resolve_to_unsigned(&self, raw: &RawValue) -> u64 {
            match raw {
                RawValue::U8(v) => *v as u64,
                RawValue::U16(v) => *v as u64,
                RawValue::U32(v) => *v as u64,
                RawValue::U64(v) => *v,
                _ => panic!(),
            }
        }

        pub fn resolve_to_signed(&self, raw: &RawValue) -> i64 {
            match raw {
                RawValue::I8(v) => *v as i64,
                RawValue::I16(v) => *v as i64,
                RawValue::I32(v) => *v as i64,
                RawValue::I64(v) => *v,
                _ => panic!(),
            }
        }
    }
}

pub type Resolver = private::Resolver<ValueStorage>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::directory_pack::{Extend, ValueStoreTrait};
    use crate::ContentAddress;
    use galvanic_test::test_suite;

    test_suite! {
        use super::*;

        mod mock {
            use super::*;
            pub struct ValueStore {}
            impl ValueStoreTrait for ValueStore {
                fn get_data(&self, id: ValueIdx, size: Option<Size>) -> Result<&[u8]> {
                    assert!(size.is_some());
                    Ok(&b"HelloWorldJubakoisawsome"[id.into_usize()..id.into_usize()+size.unwrap().into_usize()])
                }
            }

            pub struct ValueStorage {
                store: Rc<ValueStore>,
            }
            impl ValueStorage {
                pub fn new() -> Self {
                    Self {
                        store: Rc::new(ValueStore {})
                    }
                }
            }
            impl ValueStorageTrait for ValueStorage {
                type ValueStore = ValueStore;
                fn get_value_store(&self, id: ValueStoreIdx) -> Result<&Rc<Self::ValueStore>> {
                    Ok(match id.0 {
                        Idx(0) => &self.store,
                        _ => panic!(),
                    })
                }
            }
        }

        fixture storage() -> Rc<mock::ValueStorage> {
            setup(&mut self) {
                Rc::new(mock::ValueStorage::new())
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
                       extend:Some(Extend{store_id:0.into(), value_id:ValueIdx::from(10)})
                     }),
                     Value::Array("Bye Jubako".into())),
                    (RawValue::Content(ContentAddress::new(PackId::from(0), ContentIdx::from(50))),
                       Value::Content(ContentAddress::new(PackId::from(0), ContentIdx::from(50)))),
                ].into_iter()
            }
            setup(&mut self) {}
        }

        test test_resolver_resolve(storage, value) {
            let resolver = private::Resolver::new(storage.val);
            assert_eq!(&resolver.resolve(&value.params.value).unwrap(), value.params.expected);
        }

        test test_resolver_unsigned(storage) {
            let resolver = private::Resolver::new(storage.val);
            assert_eq!(resolver.resolve_to_unsigned(&RawValue::U8(0)), 0);
            assert_eq!(resolver.resolve_to_unsigned(&RawValue::U8(5)), 5);
            assert_eq!(resolver.resolve_to_unsigned(&RawValue::U8(255)), 255);
            assert_eq!(resolver.resolve_to_unsigned(&RawValue::U16(300)), 300);
            assert_eq!(resolver.resolve_to_unsigned(&RawValue::U32(30000)), 30000);
            assert_eq!(
                resolver.resolve_to_unsigned(&RawValue::U64(300000000)),
                300000000
            );
        }

        test test_resolver_signed(storage) {
            let resolver = private::Resolver::new(storage.val);
            assert_eq!(resolver.resolve_to_signed(&RawValue::I8(0)), 0);
            assert_eq!(resolver.resolve_to_signed(&RawValue::I8(5)), 5);
            assert_eq!(resolver.resolve_to_signed(&RawValue::I8(-1)), -1);
            assert_eq!(resolver.resolve_to_signed(&RawValue::I16(300)), 300);
            assert_eq!(resolver.resolve_to_signed(&RawValue::I16(-300)), -300);
            assert_eq!(resolver.resolve_to_signed(&RawValue::I32(30000)), 30000);
            assert_eq!(resolver.resolve_to_signed(&RawValue::I32(-30000)), -30000);
            assert_eq!(
                resolver.resolve_to_signed(&RawValue::I64(300000000)),
                300000000
            );
            assert_eq!(
                resolver.resolve_to_signed(&RawValue::I64(-300000000)),
                -300000000
            );
        }

        fixture indirect_value(base: Vec<u8>, extend:Option<Extend>, expected: Vec<u8>) -> RawValue {
            params {
                vec![
                    (vec![], None, vec![]),
                    ("Hello".into(), None, "Hello".into()),
                    ("Hello".into(), Some(Extend{store_id:0.into(), value_id:ValueIdx::from(0)}), "HelloHello".into()),
                    ("Hello ".into(), Some(Extend{store_id:0.into(), value_id:ValueIdx::from(10)}), "Hello Jubako".into()),
                    (vec![], Some(Extend{store_id:0.into(), value_id:ValueIdx::from(18)}), "awsome".into()),
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

        test test_resolver_indirect(storage, indirect_value) {
            let resolver = private::Resolver::new(storage.val);
            assert_eq!(
                &resolver.resolve_to_vec(&indirect_value.val).unwrap(),
                indirect_value.params.expected
            )
        }

        test test_resolver_compare(storage) {
            let resolver = private::Resolver::new(storage.val);
            let base = BaseArray::new(b"Hello ");
            let raw_value = Array {
                size: Some(Size::new(12)),
                base,
                base_len: 6,
                extend: Some(Extend{store_id:0.into(), value_id:ValueIdx::from(10)})
            };
            assert_eq!(resolver.compare_array(&raw_value, &"Hel".as_bytes()).unwrap(), cmp::Ordering::Greater);
            assert_eq!(resolver.compare_array(&raw_value, &"Hello".as_bytes()).unwrap(), cmp::Ordering::Greater);
            assert_eq!(resolver.compare_array(&raw_value, &"Hello ".as_bytes()).unwrap(), cmp::Ordering::Greater);
            assert_eq!(resolver.compare_array(&raw_value, &"Hello Jubako".as_bytes()).unwrap(), cmp::Ordering::Equal);
            assert_eq!(resolver.compare_array(&raw_value, &"Hello Jubako!".as_bytes()).unwrap(), cmp::Ordering::Less);
            assert_eq!(resolver.compare_array(&raw_value, &"Hella Jubako!".as_bytes()).unwrap(), cmp::Ordering::Greater);
            assert_eq!(resolver.compare_array(&raw_value, &"Hemmo Jubako!".as_bytes()).unwrap(), cmp::Ordering::Less);

        }
    }
}
