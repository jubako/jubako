pub mod builder;
mod entry_store;
mod finder;
mod index;
pub mod layout;
mod lazy_entry;
mod property_compare;
mod raw_layout;
mod raw_value;
mod value_store;

use self::index::IndexHeader;
use self::value_store::{ValueStore, ValueStoreTrait};
use crate::bases::*;
use crate::common::{CheckInfo, DirectoryPackHeader, Pack, PackKind};
use std::cell::Cell;
use std::io::Read;
use std::rc::Rc;
use uuid::Uuid;

pub use self::entry_store::EntryStore;
pub use self::finder::{CompareTrait, Finder};
pub use self::index::Index;
pub use self::property_compare::PropertyCompare;
pub use crate::common::{ContentAddress, Value};
pub use lazy_entry::LazyEntry;
pub use raw_value::{Array, ArrayIter, Extend, RawValue};

pub trait EntryTrait {
    fn get_variant_id(&self) -> Result<Option<VariantIdx>>;
    fn get_value(&self, idx: PropertyIdx) -> Result<RawValue>;
}

mod private {
    use super::*;
    pub trait ValueStorageTrait {
        type ValueStore: ValueStoreTrait + 'static;
        fn get_value_store(&self, id: ValueStoreIdx) -> Result<Rc<Self::ValueStore>>;
    }
}

pub struct ValueStorage(VecCache<ValueStore, DirectoryPack>);

impl ValueStorage {
    pub fn new(source: Rc<DirectoryPack>) -> Self {
        Self(VecCache::new(source))
    }
}

impl private::ValueStorageTrait for ValueStorage {
    type ValueStore = ValueStore;

    fn get_value_store(&self, store_id: ValueStoreIdx) -> Result<Rc<Self::ValueStore>> {
        Ok(Rc::clone(self.0.get(store_id)?))
    }
}

pub struct EntryStorage(VecCache<EntryStore, DirectoryPack>);

impl EntryStorage {
    pub fn new(source: Rc<DirectoryPack>) -> Self {
        Self(VecCache::new(source))
    }

    pub fn get_entry_store(&self, store_id: EntryStoreIdx) -> Result<&Rc<EntryStore>> {
        self.0.get(store_id)
    }
}

pub struct DirectoryPack {
    header: DirectoryPackHeader,
    value_stores_ptrs: ArrayReader<SizedOffset, u8>,
    entry_stores_ptrs: ArrayReader<SizedOffset, u32>,
    index_ptrs: ArrayReader<SizedOffset, u32>,
    reader: Reader,
    check_info: Cell<Option<CheckInfo>>,
}

impl DirectoryPack {
    pub fn new(reader: Reader) -> Result<DirectoryPack> {
        let reader = reader.create_sub_memory_reader(Offset::zero(), End::None)?;
        let mut flux = reader.create_flux_all();
        let header = DirectoryPackHeader::produce(&mut flux)?;
        let value_stores_ptrs = ArrayReader::new_memory_from_reader(
            &reader,
            header.value_store_ptr_pos,
            *header.value_store_count,
        )?;
        let entry_stores_ptrs = ArrayReader::new_memory_from_reader(
            &reader,
            header.entry_store_ptr_pos,
            *header.entry_store_count,
        )?;
        let index_ptrs = ArrayReader::new_memory_from_reader(
            &reader,
            header.index_ptr_pos,
            *header.index_count,
        )?;
        Ok(DirectoryPack {
            header,
            value_stores_ptrs,
            entry_stores_ptrs,
            index_ptrs,
            reader,
            check_info: Cell::new(None),
        })
    }
    pub fn get_free_data(&self) -> &[u8] {
        self.header.free_data.as_ref()
    }

    pub fn get_index(&self, index_id: IndexIdx) -> Result<Index> {
        let sized_offset = self.index_ptrs.index(*index_id)?;
        let mut index_flux = self.reader.create_flux_for(sized_offset);
        let index_header = IndexHeader::produce(&mut index_flux)?;
        let index = Index::new(index_header);
        Ok(index)
    }

    pub fn get_index_from_name(&self, index_name: &str) -> Result<Index> {
        for index_id in self.header.index_count {
            let sized_offset = self.index_ptrs.index(*index_id)?;
            let mut index_flux = self.reader.create_flux_for(sized_offset);
            let index_header = IndexHeader::produce(&mut index_flux)?;
            if index_header.name == index_name {
                let index = Index::new(index_header);
                return Ok(index);
            }
        }
        Err(format!("Cannot find index {index_name}").into())
    }

    pub fn create_value_storage(self: &Rc<Self>) -> Rc<ValueStorage> {
        Rc::new(ValueStorage::new(Rc::clone(self)))
    }

    pub fn create_entry_storage(self: &Rc<Self>) -> Rc<EntryStorage> {
        Rc::new(EntryStorage::new(Rc::clone(self)))
    }
}

impl CachableSource<ValueStore> for DirectoryPack {
    type Idx = ValueStoreIdx;
    fn get_len(&self) -> usize {
        self.header.value_store_count.into_usize()
    }

    fn get_value(&self, id: Self::Idx) -> Result<Rc<ValueStore>> {
        let sized_offset = self.value_stores_ptrs.index(*id)?;
        Ok(Rc::new(ValueStore::new(&self.reader, sized_offset)?))
    }
}

impl CachableSource<EntryStore> for DirectoryPack {
    type Idx = EntryStoreIdx;
    fn get_len(&self) -> usize {
        self.header.entry_store_count.into_usize()
    }

    fn get_value(&self, id: Self::Idx) -> Result<Rc<EntryStore>> {
        let sized_offset = self.entry_stores_ptrs.index(*id)?;
        Ok(Rc::new(EntryStore::new(&self.reader, sized_offset)?))
    }
}

impl Pack for DirectoryPack {
    fn kind(&self) -> PackKind {
        self.header.pack_header.magic
    }
    fn app_vendor_id(&self) -> u32 {
        self.header.pack_header.app_vendor_id
    }
    fn version(&self) -> (u8, u8) {
        (
            self.header.pack_header.major_version,
            self.header.pack_header.minor_version,
        )
    }
    fn uuid(&self) -> Uuid {
        self.header.pack_header.uuid
    }
    fn size(&self) -> Size {
        self.header.pack_header.file_size
    }
    fn check(&self) -> Result<bool> {
        if self.check_info.get().is_none() {
            let mut checkinfo_flux = self
                .reader
                .create_flux_from(self.header.pack_header.check_info_pos);
            let check_info = CheckInfo::produce(&mut checkinfo_flux)?;
            self.check_info.set(Some(check_info));
        }
        let mut check_flux = self
            .reader
            .create_flux_to(End::Offset(self.header.pack_header.check_info_pos));
        self.check_info
            .get()
            .unwrap()
            .check(&mut check_flux as &mut dyn Read)
    }
}

#[cfg(test)]
mod tests {
    use super::raw_value::*;
    use super::*;
    use crate::common::PackHeader;

    #[test]
    fn test_directorypackheader() {
        let mut content = vec![
            0x6a, 0x62, 0x6b, 0x64, // magic
            0x01, 0x00, 0x00, 0x00, // app_vendor_id
            0x01, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xff, // file_size
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xee, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0xdd, // index_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0x00, // entry_store_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0xaa, // value_store_ptr_pos
            0x00, 0x00, 0x00, 0x50, // index count
            0x00, 0x00, 0x00, 0x60, // entry_store count
            0x05, //value_store count
        ];
        content.extend_from_slice(&[0xff; 31]);
        let reader = Reader::from(content);
        let mut flux = reader.create_flux_all();
        assert_eq!(
            DirectoryPackHeader::produce(&mut flux).unwrap(),
            DirectoryPackHeader {
                pack_header: PackHeader {
                    magic: PackKind::Directory,
                    app_vendor_id: 0x01000000_u32,
                    major_version: 0x01_u8,
                    minor_version: 0x02_u8,
                    uuid: Uuid::from_bytes([
                        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
                        0x0c, 0x0d, 0x0e, 0x0f
                    ]),
                    file_size: Size::from(0xffff_u64),
                    check_info_pos: Offset::from(0xffee_u64),
                },
                index_ptr_pos: Offset::from(0xeedd_u64),
                entry_store_ptr_pos: Offset::from(0xee00_u64),
                value_store_ptr_pos: Offset::from(0xeeaa_u64),
                index_count: IndexCount::from(0x50_u32),
                entry_store_count: EntryStoreCount::from(0x60_u32),
                value_store_count: ValueStoreCount::from(0x05_u8),
                free_data: FreeData31::clone_from_slice(&[0xff; 31]),
            }
        );
    }

    #[derive(Debug)]
    struct FakeArray {
        size: Option<Size>,
        base: BaseArray,
        base_len: u8,
        extend: Option<ValueIdx>,
    }

    impl FakeArray {
        fn new(
            size: Option<Size>,
            base: BaseArray,
            base_len: u8,
            extend: Option<ValueIdx>,
        ) -> Self {
            Self {
                size,
                base,
                base_len,
                extend,
            }
        }
    }

    impl PartialEq<Array> for FakeArray {
        fn eq(&self, other: &Array) -> bool {
            let base = self.size == other.size
                && self.base == other.base
                && self.base_len == other.base_len;
            if !base {
                return false;
            }
            if self.extend.is_some() != other.extend.is_some() {
                return false;
            }
            if self.extend.is_none() {
                return true;
            }
            self.extend.unwrap() == other.extend.as_ref().unwrap().value_id
        }
    }

    #[test]
    fn test_directorypack() {
        let mut content = vec![
            0x6a, 0x62, 0x6b, 0x64, // magic
            0x01, 0x00, 0x00, 0x00, // app_vendor_id
            0x01, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x3C, // file_size
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x24, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x1C, // index_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFA, // entry_store_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x9C, // value_store_ptr_pos
            0x00, 0x00, 0x00, 0x01, // index count
            0x00, 0x00, 0x00, 0x01, // entry_store count
            0x01, //value_store count
        ];
        content.extend_from_slice(&[0xff; 31]); // free data
                                                // Add one value store offset 128/0x80
        content.extend_from_slice(&[
            b'H', b'e', b'l', b'l', b'o', // value 0
            b'F', b'o', b'o', // value 1
            b'J', 0xc5, 0xab, b'b', b'a', b'k', b'o', // value 2
            0x01, // kind
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // value count
            0x01, // offset_size
            0x0f, // data_size
            0x05, // Offset of entry 1
            0x08, // Offset of entry 2
        ]);
        // Add value_stores_ptr (offset 128+15+13=156/0x9C)
        content.extend_from_slice(&[
            0x00, 13, //size
            0x00, 0x00, 0x00, 0x00, 0x00, 0x8F, // Offset the tailler (128+15=143/0x8F)
        ]);
        // Add a entry_store (offset 156+8=164/0xA4)
        // One variant, with on Char1[0], a Char1[2]+Deported(1), a u24 and a content address
        #[rustfmt::skip]
        content.extend_from_slice(&[
            0x05, 0x00, 0x05, b'A', b'B', 0x01, 0x11, 0x12, 0x13, 0x00, 0x00, 0x00, 0x00, // Entry 0
            0x07, 0x02, 0x07, b'a', b'B', 0x00, 0x21, 0x22, 0x23, 0x01, 0x00, 0x00, 0x00, // Entry 1
            0x03, 0x01, 0x09, b'A', b'B', 0x02, 0x31, 0x32, 0x33, 0x00, 0x00, 0x00, 0x01, // Entry 2
            0x07, 0x02, 0x05, b'A', b'B', 0x01, 0x41, 0x42, 0x43, 0x00, 0x00, 0x00, 0x02, // Entry 3
            0x05, 0x00, 0x05, 0x00, 0x00, 0x01, 0x51, 0x52, 0x53, 0x00, 0xaa, 0xaa, 0xaa, // Entry 4
            0x00, // kind
            0x00, 0x0D, // entry size
            0x00, // variant count
            0x04, // value count
            0b0101_0001, 0b001_00000, 0x00, // Char1[0] + deported 1, idx 0x00
            0b0101_0001, 0b001_00010, 0x00, // Char1[2] + deported 1, idx 0x00
            0b0010_0010, // u24
            0b0001_0110, // content address
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x41, // data size
        ]);
        // Add a entry_store_ptr (offset 164+65+21=250/0xFA)
        content.extend_from_slice(&[
            0x00, 21, // size
            0x00, 0x00, 0x00, 0x00, 0x00, 0xE5, // offset of the tailler (164+65=229/0xE5)
        ]);
        // Add one index (offset 250+8=258/0x102)
        content.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x00, // store_id
            0x00, 0x00, 0x00, 0x04, // entry_count (use only 4 from the 5 available)
            0x00, 0x00, 0x00, 0x01, // entry offset (skip the first one)
            0x00, 0x00, 0x00, 0x00, // extra_data
            0x00, // index_property (use the first pstring a binary search property
            0x08, b'm', b'y', b' ', b'i', b'n', b'd', b'e', b'x', // Pstring "my index"
        ]);
        // Add a index_ptr (offset 258+26=284/0x11C)
        content.extend_from_slice(&[
            0x00, 26, //size
            0x00, 0x00, 0x00, 0x00, 0x01, 0x02, // offset
        ]);
        // end = 284+8=292/0x124
        let hash = blake3::hash(&content);
        content.push(0x01); // check info off: 284
        content.extend(hash.as_bytes()); // end : 284+32 = 316/0x13C
        let directory_pack = Rc::new(DirectoryPack::new(content.into()).unwrap());
        let index = directory_pack.get_index(0.into()).unwrap();
        let value_storage = directory_pack.create_value_storage();
        let entry_storage = directory_pack.create_entry_storage();
        let builder = builder::AnyBuilder::new(
            index.get_store(&entry_storage).unwrap(),
            value_storage.as_ref(),
        )
        .unwrap();
        let finder = Finder::new(&index);
        assert_eq!(index.entry_count(), 4.into());
        {
            let entry = finder.get_entry(&builder, 0.into()).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            let value0 = entry.get_value(0.into()).unwrap();
            if let RawValue::Array(a) = &value0 {
                assert_eq!(
                    &FakeArray::new(
                        Some(Size::new(7)),
                        BaseArray::default(),
                        0,
                        Some(ValueIdx::from(2))
                    ),
                    a
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                value0.as_vec().unwrap(),
                b"J\xc5\xabbako" // Jūbako
            );
            let value1 = entry.get_value(1.into()).unwrap();
            if let RawValue::Array(a) = &value1 {
                assert_eq!(
                    &FakeArray::new(
                        Some(Size::new(7)),
                        BaseArray::new(b"aB"),
                        2,
                        Some(ValueIdx::from(0))
                    ),
                    a
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(value1.as_vec().unwrap(), b"aBHello");
            assert_eq!(entry.get_value(2.into()).unwrap(), RawValue::U32(0x212223));
            assert_eq!(
                entry.get_value(3.into()).unwrap(),
                RawValue::Content(ContentAddress {
                    pack_id: PackId::from(1),
                    content_id: ContentIdx::from(0)
                })
            );
        }
        {
            let entry = finder.get_entry(&builder, 1.into()).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            let value0 = entry.get_value(0.into()).unwrap();
            if let RawValue::Array(a) = &value0 {
                assert_eq!(
                    &FakeArray::new(
                        Some(Size::new(3)),
                        BaseArray::default(),
                        0,
                        Some(ValueIdx::from(1))
                    ),
                    a
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(value0.as_vec().unwrap(), b"Foo");
            let value1 = entry.get_value(1.into()).unwrap();
            if let RawValue::Array(a) = &value1 {
                assert_eq!(
                    &FakeArray::new(
                        Some(Size::new(9)),
                        BaseArray::new(b"AB"),
                        2,
                        Some(ValueIdx::from(2))
                    ),
                    a
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(value1.as_vec().unwrap(), b"ABJ\xc5\xabbako");
            assert_eq!(entry.get_value(2.into()).unwrap(), RawValue::U32(0x313233));
            assert_eq!(
                entry.get_value(3.into()).unwrap(),
                RawValue::Content(ContentAddress {
                    pack_id: PackId::from(0),
                    content_id: ContentIdx::from(1)
                })
            );
        }
        {
            let entry = finder.get_entry(&builder, 2.into()).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            let value0 = entry.get_value(0.into()).unwrap();
            if let RawValue::Array(a) = &value0 {
                assert_eq!(
                    &FakeArray::new(
                        Some(Size::new(7)),
                        BaseArray::default(),
                        0,
                        Some(ValueIdx::from(2))
                    ),
                    a
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(value0.as_vec().unwrap(), b"J\xc5\xabbako");
            let value1 = entry.get_value(1.into()).unwrap();
            if let RawValue::Array(a) = &value1 {
                assert_eq!(
                    &FakeArray::new(
                        Some(Size::new(5)),
                        BaseArray::new(b"AB"),
                        2,
                        Some(ValueIdx::from(1))
                    ),
                    a
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(value1.as_vec().unwrap(), b"ABFoo");
            assert_eq!(entry.get_value(2.into()).unwrap(), RawValue::U32(0x414243));
            assert_eq!(
                entry.get_value(3.into()).unwrap(),
                RawValue::Content(ContentAddress {
                    pack_id: PackId::from(0),
                    content_id: ContentIdx::from(2)
                })
            );
        }
        {
            let entry = finder.get_entry(&builder, 3.into()).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            let value0 = entry.get_value(0.into()).unwrap();
            if let RawValue::Array(a) = &value0 {
                assert_eq!(
                    &FakeArray::new(
                        Some(Size::new(5)),
                        BaseArray::default(),
                        0,
                        Some(ValueIdx::from(0))
                    ),
                    a
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(value0.as_vec().unwrap(), b"Hello");
            let value1 = entry.get_value(1.into()).unwrap();
            if let RawValue::Array(a) = &value1 {
                assert_eq!(
                    &FakeArray::new(
                        Some(Size::new(5)),
                        BaseArray::default(),
                        2,
                        Some(ValueIdx::from(1))
                    ),
                    a
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(value1.as_vec().unwrap(), b"\0\0Foo");
            assert_eq!(entry.get_value(2.into()).unwrap(), RawValue::U32(0x515253));
            assert_eq!(
                entry.get_value(3.into()).unwrap(),
                RawValue::Content(ContentAddress {
                    pack_id: PackId::from(0),
                    content_id: ContentIdx::from(0xaaaaaa)
                })
            );
        }
    }
}
