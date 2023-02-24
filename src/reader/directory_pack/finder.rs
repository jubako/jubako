use super::builder::BuilderTrait;
use crate::bases::*;
use std::cmp::Ordering;
use std::rc::Rc;

pub trait CompareTrait {
    fn compare_entry(&self, idx: EntryIdx) -> Result<Ordering>;
}

pub struct Finder<Builder: BuilderTrait> {
    builder: Rc<Builder>,
    offset: EntryIdx,
    count: EntryCount,
}

impl<Builder: BuilderTrait> Finder<Builder> {
    pub fn new(builder: Rc<Builder>, offset: EntryIdx, count: EntryCount) -> Self {
        Self {
            builder,
            offset,
            count,
        }
    }

    fn _get_entry(&self, id: EntryIdx) -> Result<Builder::Entry> {
        self.builder.create_entry(self.offset + id)
    }

    pub fn offset(&self) -> EntryIdx {
        self.offset
    }

    pub fn count(&self) -> EntryCount {
        self.count
    }

    pub fn builder(&self) -> &Rc<Builder> {
        &self.builder
    }

    pub fn get_entry(&self, id: EntryIdx) -> Result<Builder::Entry> {
        if id.is_valid(self.count) {
            self._get_entry(id)
        } else {
            Err("Invalid id".to_string().into())
        }
    }

    pub fn find<Comparator: CompareTrait>(
        &self,
        comparator: &Comparator,
    ) -> Result<Option<EntryIdx>> {
        for idx in self.count {
            let cmp = comparator.compare_entry(self.offset + idx)?;
            if cmp.is_eq() {
                return Ok(Some(idx));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::directory_pack::{builder, schema};
    use crate::reader::directory_pack::{EntryStore, EntryTrait};
    use crate::reader::RawValue;
    use std::rc::Rc;

    mod mock {
        use super::*;
        use crate::reader::directory_pack::private::ValueStorageTrait;
        use crate::reader::directory_pack::value_store::ValueStoreTrait;
        #[derive(PartialEq, Eq, Debug)]
        pub struct Entry {
            v: RawValue,
        }
        impl Entry {
            pub fn new(v: u16) -> Self {
                let v = RawValue::U16(v);
                Self { v }
            }
        }
        impl EntryTrait for Entry {
            fn get_variant_id(&self) -> Result<Option<VariantIdx>> {
                Ok(None)
            }
            fn get_value(&self, idx: PropertyIdx) -> Result<RawValue> {
                Ok(match idx {
                    PropertyIdx(Idx(0)) => self.v.clone(),
                    _ => panic!(),
                })
            }
        }

        pub struct EntryCompare {
            reference: u32,
        }

        impl EntryCompare {
            pub fn new(reference: u32) -> Self {
                Self { reference }
            }
        }

        impl CompareTrait for EntryCompare {
            fn compare_entry(&self, index: EntryIdx) -> Result<Ordering> {
                // In our mock schema, the value stored in the entry is equal to the index.
                Ok(index.into_u32().cmp(&self.reference))
            }
        }

        pub struct Builder {}
        impl builder::BuilderTrait for Builder {
            type Entry = Entry;
            fn create_entry(&self, idx: EntryIdx) -> Result<Self::Entry> {
                Ok(Entry::new(idx.into_u16()))
            }
        }

        pub struct Schema {}

        impl schema::SchemaTrait for Schema {
            type Builder = Builder;
            type ValueStorage = ValueStorage;
            fn create_builder(
                _store: Rc<EntryStore>,
                _value_storage: &ValueStorage,
            ) -> Result<Rc<Self::Builder>> {
                unreachable!()
            }
        }

        #[derive(Debug)]
        pub struct ValueStore {}
        impl ValueStoreTrait for ValueStore {
            fn get_data(&self, _id: ValueIdx, _size: Option<Size>) -> Result<&[u8]> {
                unreachable!()
            }
        }

        pub struct ValueStorage {}
        impl ValueStorageTrait for ValueStorage {
            type ValueStore = ValueStore;
            fn get_value_store(&self, _id: ValueStoreIdx) -> Result<Rc<Self::ValueStore>> {
                unreachable!()
            }
        }
    }

    #[test]
    fn test_finder() {
        let builder = Rc::new(mock::Builder {});
        let finder = Finder::new(builder, EntryIdx::from(0), EntryCount::from(10));

        for i in 0..10 {
            let entry = finder.get_entry(i.into()).unwrap();
            let value0 = entry.get_value(0.into()).unwrap();
            assert_eq!(value0.as_unsigned(), i as u64);
        }

        for i in 0..10 {
            let comparator = mock::EntryCompare::new(i);
            let idx = finder.find(&comparator).unwrap().unwrap();
            let entry = finder.get_entry(idx).unwrap();
            let value0 = entry.get_value(0.into()).unwrap();
            assert_eq!(value0.as_unsigned(), i as u64);
        }

        let comparator = mock::EntryCompare::new(10);
        let result = finder.find(&comparator).unwrap();
        assert_eq!(result, None);
    }
}
