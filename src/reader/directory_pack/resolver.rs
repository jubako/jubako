use super::key_store::KeyStoreTrait;
use super::private::KeyStorageTrait;
use super::{Array, Content, DirectoryPack, Extend, RawValue};
use crate::bases::*;
use crate::common::Value;
use std::cell::OnceCell;
use std::rc::Rc;

pub(crate) mod private {
    use super::*;
    pub struct Resolver<K: KeyStorageTrait> {
        directory: Rc<K>,
        stores: Vec<OnceCell<K::KeyStore>>,
    }

    impl<K: KeyStorageTrait> Resolver<K> {
        pub fn new(directory: Rc<K>) -> Self {
            let mut stores = Vec::new();
            stores.resize_with(directory.get_key_store_count().0 as usize, Default::default);
            Self { directory, stores }
        }

        fn get_key_store(&self, id: Idx<u8>) -> Result<&K::KeyStore> {
            self.stores[id.0 as usize].get_or_try_init(|| self._get_key_store(id))
        }

        fn _get_key_store(&self, id: Idx<u8>) -> Result<K::KeyStore> {
            self.directory.get_key_store(id)
        }

        fn get_data(&self, extend: &Extend) -> Result<Vec<u8>> {
            let key_store = self.get_key_store(extend.store_id)?;
            key_store.get_data(extend.key_id.into())
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
            pub struct KeyStore {}
            impl KeyStoreTrait for KeyStore {
                fn get_data(&self, id: Idx<u64>) -> Result<Vec<u8>> {
                    Ok(match id {
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

            pub struct KeyStorage {}
            impl KeyStorageTrait for KeyStorage {
                type KeyStore = KeyStore;
                fn get_key_store_count(&self) -> Count<u8> {
                    Count(1)
                }
                fn get_key_store(&self, id: Idx<u8>) -> Result<Self::KeyStore> {
                    Ok(match id {
                        Idx(0) => KeyStore {},
                        _ => panic!(),
                    })
                }
            }
        }

        fixture resolver() -> private::Resolver<mock::KeyStorage> {
            setup(&mut self) {
                let key_storage = Rc::new(mock::KeyStorage {});
                private::Resolver::new(key_storage)
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
                    (RawValue::Array(Array{base:"Bye ".into(), extend:Some(Extend{store_id:Idx(0), key_id:2})}),
                       Value::Array("Bye Jubako".into())),
                    (RawValue::Content(Content::new(ContentAddress::new(Id(0), Idx(50)), None)),
                       Value::Content(Content::new(ContentAddress::new(Id(0), Idx(50)), None))),
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
                    ("Hello".into(), Some(Extend{store_id:Idx(0), key_id:0}), "HelloHello".into()),
                    ("Hello ".into(), Some(Extend{store_id:Idx(0), key_id:2}), "Hello Jubako".into()),
                    (vec![], Some(Extend{store_id:Idx(0), key_id:4}), "awsome".into()),
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
