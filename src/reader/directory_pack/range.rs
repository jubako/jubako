use super::builder::BuilderTrait;
use crate::bases::*;
use std::cmp::Ordering;

pub trait CompareTrait {
    fn compare_entry(&self, idx: EntryIdx) -> Result<Ordering>;
}

pub trait RangeTrait {
    fn count(&self) -> EntryCount;
    fn offset(&self) -> EntryIdx;

    fn get_entry<Builder: BuilderTrait>(
        &self,
        builder: &Builder,
        id: EntryIdx,
    ) -> Result<Builder::Entry> {
        if id.is_valid(self.count()) {
            builder.create_entry(self.offset() + id)
        } else {
            Err("Invalid id".to_string().into())
        }
    }

    fn find<Comparator: CompareTrait>(&self, comparator: &Comparator) -> Result<Option<EntryIdx>> {
        for idx in self.count() {
            let cmp = comparator.compare_entry(self.offset() + idx)?;
            if cmp.is_eq() {
                return Ok(Some(idx));
            }
        }
        Ok(None)
    }
}

impl RangeTrait for EntryRange {
    fn count(&self) -> EntryCount {
        self.size()
    }

    fn offset(&self) -> EntryIdx {
        self.begin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::directory_pack::builder;
    use crate::reader::directory_pack::EntryTrait;
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
        let builder = mock::Builder {};
        let range = EntryRange::new_from_size(EntryIdx::from(0), EntryCount::from(10));

        for i in 0..10 {
            let entry = range.get_entry(&builder, i.into()).unwrap();
            let value0 = entry.get_value(0.into()).unwrap();
            assert_eq!(value0.as_unsigned(), i as u64);
        }

        for i in 0..10 {
            let comparator = mock::EntryCompare::new(i);
            let idx = range.find(&comparator).unwrap().unwrap();
            let entry = range.get_entry(&builder, idx).unwrap();
            let value0 = entry.get_value(0.into()).unwrap();
            assert_eq!(value0.as_unsigned(), i as u64);
        }

        let comparator = mock::EntryCompare::new(10);
        let result = range.find(&comparator).unwrap();
        assert_eq!(result, None);
    }
}
