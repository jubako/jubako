use super::builder::BuilderTrait;
use crate::bases::*;
use std::cmp::Ordering;

pub trait CompareTrait {
    fn ordered(&self) -> bool;
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
        if comparator.ordered() {
            // INVARIANTS:
            // - 0 <= left <= left + size = right <= self.count()
            // - comparator returns Less for everything in self[..left]
            // - comparator returns Greater for everything in self[right..]
            let mut size = self.count();
            let mut left = EntryIdx::from(0);
            let mut right = left + size;
            while left < right {
                let mid = left + size / 2;

                // SAFETY: the while condition means `size` is strictly positive, so
                // `size/2 < size`. Thus `left + size/2 < left + size`, which
                // coupled with the `left + size <= self.len()` invariant means
                // we have `left + size/2 < self.len()`, and this is in-bounds.
                let cmp = comparator.compare_entry(self.offset() + mid)?;

                // The reason why we use if/else control flow rather than match
                // is because match reorders comparison operations, which is perf sensitive.
                // This is x86 asm for u8: https://rust.godbolt.org/z/8Y8Pra.
                if cmp == Ordering::Less {
                    left = mid + EntryCount::from(1);
                } else if cmp == Ordering::Greater {
                    right = mid;
                } else {
                    return Ok(Some(mid));
                }

                size = right - left;
            }
            Ok(None)
        } else {
            for idx in self.count() {
                let cmp = comparator.compare_entry(self.offset() + idx)?;
                if cmp.is_eq() {
                    return Ok(Some(idx));
                }
            }
            Ok(None)
        }
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
    use std::sync::Arc;

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
            fn get_value(&self, name: &str) -> Result<RawValue> {
                if name == "foo" {
                    Ok(self.v.clone())
                } else {
                    panic!()
                }
            }
        }

        pub struct EntryCompare {
            reference: u32,
            ordered: bool,
        }

        impl EntryCompare {
            pub fn new(reference: u32, ordered: bool) -> Self {
                Self { reference, ordered }
            }
        }

        impl CompareTrait for EntryCompare {
            fn compare_entry(&self, index: EntryIdx) -> Result<Ordering> {
                // In our mock schema, the value stored in the entry is equal to the index.
                Ok(index.into_u32().cmp(&self.reference))
            }
            fn ordered(&self) -> bool {
                self.ordered
            }
        }

        pub struct Builder {}
        impl builder::BuilderTrait for Builder {
            type Entry = Entry;
            fn create_entry(&self, idx: EntryIdx) -> Result<Self::Entry> {
                Ok(Entry::new(idx.into_u32() as u16))
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
            fn get_value_store(&self, _id: ValueStoreIdx) -> Result<Arc<Self::ValueStore>> {
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
            let value0 = entry.get_value("foo").unwrap();
            assert_eq!(value0.as_unsigned(), i as u64);
        }
    }

    #[test]
    fn test_comparator_false() {
        let builder = mock::Builder {};
        let range = EntryRange::new_from_size(EntryIdx::from(0), EntryCount::from(10));

        for i in 0..10 {
            let comparator = mock::EntryCompare::new(i, false);
            let idx = range.find(&comparator).unwrap().unwrap();
            let entry = range.get_entry(&builder, idx).unwrap();
            let value0 = entry.get_value("foo").unwrap();
            assert_eq!(value0.as_unsigned(), i as u64);
        }

        let comparator = mock::EntryCompare::new(10, false);
        let result = range.find(&comparator).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_comparator_true() {
        let builder = mock::Builder {};
        let range = EntryRange::new_from_size(EntryIdx::from(0), EntryCount::from(10));

        for i in 0..10 {
            let comparator = mock::EntryCompare::new(i, true);
            let idx = range.find(&comparator).unwrap().unwrap();
            let entry = range.get_entry(&builder, idx).unwrap();
            let value0 = entry.get_value("foo").unwrap();
            assert_eq!(value0.as_unsigned(), i as u64);
        }

        let comparator = mock::EntryCompare::new(10, true);
        let result = range.find(&comparator).unwrap();
        assert_eq!(result, None);
    }
}
