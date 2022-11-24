use super::private::ValueStorageTrait;
use super::value_store::ValueStoreTrait;
use super::{Array, Content, DirectoryPack, Extend, RawValue};
use crate::bases::*;
use crate::common::Value;
use std::cell::OnceCell;
use std::cmp;
use std::rc::Rc;

pub(crate) mod private {
    use super::*;
    pub struct Resolver<K: ValueStorageTrait> {
        directory: Rc<K>,
        stores: Vec<OnceCell<K::ValueStore>>,
    }

    impl<K: ValueStorageTrait> Resolver<K> {
        pub fn new(directory: Rc<K>) -> Self {
            let mut stores = Vec::new();
            stores.resize_with(
                directory.get_value_store_count().into_usize(),
                Default::default,
            );
            Self { directory, stores }
        }

        fn get_value_store(&self, id: ValueStoreIdx) -> Result<&K::ValueStore> {
            self.stores[id.into_usize()].get_or_try_init(|| self._get_value_store(id))
        }

        fn _get_value_store(&self, id: ValueStoreIdx) -> Result<K::ValueStore> {
            self.directory.get_value_store(id)
        }

        fn get_data(&self, extend: &Extend) -> Result<Vec<u8>> {
            let value_store = self.get_value_store(extend.store_id)?;
            value_store.get_data(extend.value_id)
        }

        fn resolve_array_to_vec(&self, array: &Array) -> Result<Vec<u8>> {
            Ok(match &array.extend {
                None => array.base.clone(),
                Some(e) => {
                    let data = self.get_data(e)?;
                    [array.base.as_slice(), data.as_slice()].concat()
                }
            })
        }

        pub fn resolve(&self, raw: &RawValue) -> Result<Value> {
            Ok(match raw {
                RawValue::Content(c) => Value::Content(c.clone()),
                RawValue::U8(v) => Value::Unsigned(*v as u64),
                RawValue::U16(v) => Value::Unsigned(*v as u64),
                RawValue::U32(v) => Value::Unsigned(*v as u64),
                RawValue::U64(v) => Value::Unsigned(*v as u64),
                RawValue::I8(v) => Value::Signed(*v as i64),
                RawValue::I16(v) => Value::Signed(*v as i64),
                RawValue::I32(v) => Value::Signed(*v as i64),
                RawValue::I64(v) => Value::Signed(*v as i64),
                RawValue::Array(a) => Value::Array(self.resolve_array_to_vec(a)?),
            })
        }

        pub fn compare(&self, raw: &RawValue, value: &Value) -> Result<cmp::Ordering> {
            match value {
                Value::Content(_) => Err("Content cannot be compared.".to_string().into()),
                Value::Unsigned(v) => match raw {
                    RawValue::U8(r) => Ok((*r as u64).cmp(v)),
                    RawValue::U16(r) => Ok((*r as u64).cmp(v)),
                    RawValue::U32(r) => Ok((*r as u64).cmp(v)),
                    RawValue::U64(r) => Ok((*r as u64).cmp(v)),
                    _ => Err("Values kind cannot be compared.".to_string().into()),
                },
                Value::Signed(v) => match raw {
                    RawValue::I8(r) => Ok((*r as i64).cmp(v)),
                    RawValue::I16(r) => Ok((*r as i64).cmp(v)),
                    RawValue::I32(r) => Ok((*r as i64).cmp(v)),
                    RawValue::I64(r) => Ok((*r as i64).cmp(v)),
                    _ => Err("Values kind cannot be compared.".to_string().into()),
                },
                Value::Array(v) => match raw {
                    RawValue::Array(a) => {
                        let cmp = a.base.as_slice().cmp(&v[..a.base.len()]);
                        if cmp.is_ne() {
                            Ok(cmp)
                        } else {
                            match &a.extend {
                                None => Ok(cmp),
                                Some(e) => {
                                    let d = self.get_data(e)?;
                                    Ok(d.as_slice().cmp(&v[a.base.len()..]))
                                }
                            }
                        }
                    }
                    _ => Err("Values kind cannot be compared.".to_string().into()),
                },
            }
        }

        pub fn resolve_to_vec(&self, raw: &RawValue) -> Result<Vec<u8>> {
            if let RawValue::Array(a) = raw {
                self.resolve_array_to_vec(a)
            } else {
                panic!();
            }
        }

        pub fn resolve_to_content<'a>(&self, raw: &'a RawValue) -> &'a Content {
            if let RawValue::Content(c) = raw {
                c
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

pub type Resolver = private::Resolver<DirectoryPack>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContentAddress;
    use galvanic_test::test_suite;

    test_suite! {
        use super::*;

        mod mock {
            use super::*;
            pub struct ValueStore {}
            impl ValueStoreTrait for ValueStore {
                fn get_data(&self, id: ValueIdx) -> Result<Vec<u8>> {
                    Ok(match *id {
                        Idx(0) => "Hello",
                        Idx(1) => "World",
                        Idx(2) => "Jubako",
                        Idx(3) => "is",
                        Idx(4) => "awsome",
                        _ => panic!(),
                    }
                    .as_bytes()
                    .to_vec())
                }
            }

            pub struct ValueStorage {}
            impl ValueStorageTrait for ValueStorage {
                type ValueStore = ValueStore;
                fn get_value_store_count(&self) -> ValueStoreCount {
                    1.into()
                }
                fn get_value_store(&self, id: ValueStoreIdx) -> Result<Self::ValueStore> {
                    Ok(match id {
                        ValueStoreIdx(Idx(0)) => ValueStore {},
                        _ => panic!(),
                    })
                }
            }
        }

        fixture resolver() -> private::Resolver<mock::ValueStorage> {
            setup(&mut self) {
                let value_storage = Rc::new(mock::ValueStorage {});
                private::Resolver::new(value_storage)
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
                    (RawValue::Array(Array{base:"Bye ".into(), extend:Some(Extend{store_id:0.into(), value_id:ValueIdx::from(2)})}),
                       Value::Array("Bye Jubako".into())),
                    (RawValue::Content(Content::new(ContentAddress::new(PackId::from(0), ContentIdx::from(50)), None)),
                       Value::Content(Content::new(ContentAddress::new(PackId::from(0), ContentIdx::from(50)), None))),
                ].into_iter()
            }
            setup(&mut self) {}
        }

        test test_resolver_resolve(resolver, value) {
            let resolver = resolver.val;
            assert_eq!(&resolver.resolve(&value.params.value).unwrap(), value.params.expected);
        }

        test test_resolver_unsigned(resolver) {
            let resolver = resolver.val;
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

        test test_resolver_signed(resolver) {
            let resolver = resolver.val;
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
                    ("Hello ".into(), Some(Extend{store_id:0.into(), value_id:ValueIdx::from(2)}), "Hello Jubako".into()),
                    (vec![], Some(Extend{store_id:0.into(), value_id:ValueIdx::from(4)}), "awsome".into()),
                ].into_iter()
            }
            setup(&mut self) {
                RawValue::Array(Array {
                    base: self.base.clone(),
                    extend: self.extend.clone()
                })
            }
        }

        test test_resolver_indirect(resolver, indirect_value) {
            let resolver = resolver.val;
            assert_eq!(
                &resolver.resolve_to_vec(&indirect_value.val).unwrap(),
                indirect_value.params.expected
            )
        }
    }
}
