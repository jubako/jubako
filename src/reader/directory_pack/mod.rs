pub mod builder;
mod entry_store;
mod finder;
mod index;
pub mod layout;
mod lazy_entry;
mod property_compare;
mod raw_layout;
mod raw_value;
mod resolver;
pub mod schema;
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
pub use self::property_compare::AnyPropertyCompare;
pub use crate::common::{Content, Value};
pub use lazy_entry::LazyEntry;
pub use raw_value::{Array, Extend, RawValue};
pub use resolver::Resolver;
pub use schema::AnySchema;

pub trait EntryTrait {
    fn get_variant_id(&self) -> VariantIdx;
    fn get_value(&self, idx: PropertyIdx) -> Result<RawValue>;
}

mod private {
    use super::*;
    pub trait ValueStorageTrait {
        type ValueStore: ValueStoreTrait;
        fn get_value_store(&self, id: ValueStoreIdx) -> Result<&Rc<Self::ValueStore>>;
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

    fn get_value_store(&self, store_id: ValueStoreIdx) -> Result<&Rc<Self::ValueStore>> {
        self.0.get(store_id)
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
        let mut stream = reader.create_stream_all();
        let header = DirectoryPackHeader::produce(&mut stream)?;
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
        let mut index_stream = self.reader.create_stream_for(sized_offset);
        let index_header = IndexHeader::produce(&mut index_stream)?;
        let index = Index::new(index_header);
        Ok(index)
    }

    pub fn get_index_from_name(&self, index_name: &str) -> Result<Index> {
        for index_id in self.header.index_count {
            let sized_offset = self.index_ptrs.index(*index_id)?;
            let mut index_stream = self.reader.create_stream_for(sized_offset);
            let index_header = IndexHeader::produce(&mut index_stream)?;
            if index_header.name == index_name {
                let index = Index::new(index_header);
                return Ok(index);
            }
        }
        Err("Cannot find index".to_string().into())
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
            let mut checkinfo_stream = self
                .reader
                .create_stream_from(self.header.pack_header.check_info_pos);
            let check_info = CheckInfo::produce(&mut checkinfo_stream)?;
            self.check_info.set(Some(check_info));
        }
        let mut check_stream = self
            .reader
            .create_stream_to(End::Offset(self.header.pack_header.check_info_pos));
        self.check_info
            .get()
            .unwrap()
            .check(&mut check_stream as &mut dyn Read)
    }
}

#[cfg(test)]
mod tests {
    use super::raw_value::*;
    use super::*;
    use crate::common::{ContentAddress, PackHeader};
    use crate::reader::schema::SchemaTrait;

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
        let reader = Reader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        assert_eq!(
            DirectoryPackHeader::produce(&mut stream).unwrap(),
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
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x39, // file_size
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x19, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x11, // index_ptr_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xEF, // entry_store_ptr_pos
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
        // One variant, with on PString, a 2ArrayChar/Pstring, a u24 and a content address
        #[rustfmt::skip]
        content.extend_from_slice(&[
            0x00, 0x01, b'A', b'B', 0x11, 0x12, 0x13, 0x00, 0x00, 0x00, 0x00, // Entry 0
            0x02, 0x00, b'a', b'B', 0x21, 0x22, 0x23, 0x01, 0x00, 0x00, 0x00, // Entry 1
            0x01, 0x02, b'A', b'B', 0x31, 0x32, 0x33, 0x00, 0x00, 0x00, 0x01, // Entry 2
            0x02, 0x01, b'A', b'B', 0x41, 0x42, 0x43, 0x00, 0x00, 0x00, 0x02, // Entry 3
            0x00, 0x01, 0x00, 0x00, 0x51, 0x52, 0x53, 0x00, 0xaa, 0xaa, 0xaa, // Entry 4
            0x00, // kind
            0x00, 0x0B, // entry size
            0x01, // variant count
            0x05, // value count
            0b0110_0000, 0x00, // Pstring(1), idx 0x00
            0b0111_0000, 0x00,        // Psstringlookup(1), idx 0x00
            0b0100_0001, // char[2]
            0b0010_0010, // u24
            0b0001_0000, // content address
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x37, // data size
        ]);
        // Add a entry_store_ptr (offset 164+55+20=239/0xEF)
        content.extend_from_slice(&[
            0x00, 20, // size
            0x00, 0x00, 0x00, 0x00, 0x00, 0xDB, // offset of the tailler (164+55=219/0xDB)
        ]);
        // Add one index (offset 239+8=247/0xF7)
        content.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x00, // store_id
            0x00, 0x00, 0x00, 0x04, // entry_count (use only 4 from the 5 available)
            0x00, 0x00, 0x00, 0x01, // entry offset (skip the first one)
            0x00, 0x00, 0x00, 0x00, // extra_data
            0x00, // index_property (use the first pstring a binary search property
            0x08, b'm', b'y', b' ', b'i', b'n', b'd', b'e', b'x', // Pstring "my index"
        ]);
        // Add a index_ptr (offset 247+26=273/0x111)
        content.extend_from_slice(&[
            0x00, 26, //size
            0x00, 0x00, 0x00, 0x00, 0x00, 0xF7, // offset
        ]);
        // end = 273+8=281/0x119
        let hash = blake3::hash(&content);
        content.push(0x01); // check info off: 281
        content.extend(hash.as_bytes()); // end : 281+32 = 313/0x139
        let reader = Reader::new(content, End::None);
        let directory_pack = Rc::new(DirectoryPack::new(reader).unwrap());
        let index = directory_pack.get_index(0.into()).unwrap();
        let value_storage = directory_pack.create_value_storage();
        let entry_storage = directory_pack.create_entry_storage();
        let resolver = Resolver::new(value_storage);
        let schema = schema::AnySchema {};
        let builder = schema
            .create_builder(index.get_store(&entry_storage).unwrap())
            .unwrap();
        let finder: Finder<schema::AnySchema> = index.get_finder(&builder).unwrap();
        assert_eq!(index.entry_count(), 4.into());
        {
            let entry = finder.get_entry(0.into()).unwrap();
            assert_eq!(entry.get_variant_id().into_u8(), 0);
            let value0 = entry.get_value(0.into()).unwrap();
            if let RawValue::Array(a) = &value0 {
                assert_eq!(
                    a,
                    &Array::new(Vec::new(), Some(Extend::new(0.into(), ValueIdx::from(2))))
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                resolver.resolve_to_vec(&value0).unwrap(),
                b"J\xc5\xabbako" // Jūbako
            );
            let value1 = entry.get_value(1.into()).unwrap();
            if let RawValue::Array(a) = &value1 {
                assert_eq!(
                    a,
                    &Array::new(
                        vec![b'a', b'B'],
                        Some(Extend::new(0.into(), ValueIdx::from(0)))
                    )
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(resolver.resolve_to_vec(&value1).unwrap(), b"aBHello");
            assert_eq!(entry.get_value(2.into()).unwrap(), RawValue::U32(0x212223));
            assert_eq!(
                entry.get_value(3.into()).unwrap(),
                RawValue::Content(Content::new(
                    ContentAddress {
                        pack_id: PackId::from(1),
                        content_id: ContentIdx::from(0)
                    },
                    None
                ))
            );
        }
        {
            let entry = finder.get_entry(1.into()).unwrap();
            assert_eq!(entry.get_variant_id().into_u8(), 0);
            let value0 = entry.get_value(0.into()).unwrap();
            if let RawValue::Array(a) = &value0 {
                assert_eq!(
                    a,
                    &Array::new(Vec::new(), Some(Extend::new(0.into(), ValueIdx::from(1))))
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(resolver.resolve_to_vec(&value0).unwrap(), b"Foo");
            let value1 = entry.get_value(1.into()).unwrap();
            if let RawValue::Array(a) = &value1 {
                assert_eq!(
                    a,
                    &Array::new(
                        vec![b'A', b'B'],
                        Some(Extend::new(0.into(), ValueIdx::from(2)))
                    )
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                resolver.resolve_to_vec(&value1).unwrap(),
                b"ABJ\xc5\xabbako"
            );
            assert_eq!(entry.get_value(2.into()).unwrap(), RawValue::U32(0x313233));
            assert_eq!(
                entry.get_value(3.into()).unwrap(),
                RawValue::Content(Content::new(
                    ContentAddress {
                        pack_id: PackId::from(0),
                        content_id: ContentIdx::from(1)
                    },
                    None
                ))
            );
        }
        {
            let entry = finder.get_entry(2.into()).unwrap();
            assert_eq!(entry.get_variant_id().into_u8(), 0);
            let value0 = entry.get_value(0.into()).unwrap();
            if let RawValue::Array(a) = &value0 {
                assert_eq!(
                    a,
                    &Array::new(Vec::new(), Some(Extend::new(0.into(), ValueIdx::from(2))))
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(resolver.resolve_to_vec(&value0).unwrap(), b"J\xc5\xabbako");
            let value1 = entry.get_value(1.into()).unwrap();
            if let RawValue::Array(a) = &value1 {
                assert_eq!(
                    a,
                    &Array::new(
                        vec![b'A', b'B'],
                        Some(Extend::new(0.into(), ValueIdx::from(1)))
                    )
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(resolver.resolve_to_vec(&value1).unwrap(), b"ABFoo");
            assert_eq!(entry.get_value(2.into()).unwrap(), RawValue::U32(0x414243));
            assert_eq!(
                entry.get_value(3.into()).unwrap(),
                RawValue::Content(Content::new(
                    ContentAddress {
                        pack_id: PackId::from(0),
                        content_id: ContentIdx::from(2)
                    },
                    None
                ))
            );
        }
        {
            let entry = finder.get_entry(3.into()).unwrap();
            assert_eq!(entry.get_variant_id().into_u8(), 0);
            let value0 = entry.get_value(0.into()).unwrap();
            if let RawValue::Array(a) = &value0 {
                assert_eq!(
                    a,
                    &Array::new(Vec::new(), Some(Extend::new(0.into(), ValueIdx::from(0))))
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(resolver.resolve_to_vec(&value0).unwrap(), b"Hello");
            let value1 = entry.get_value(1.into()).unwrap();
            if let RawValue::Array(a) = &value1 {
                assert_eq!(
                    a,
                    &Array::new(vec![0, 0], Some(Extend::new(0.into(), ValueIdx::from(1))))
                );
            } else {
                panic!("Must be a array");
            };
            assert_eq!(resolver.resolve_to_vec(&value1).unwrap(), b"\0\0Foo");
            assert_eq!(entry.get_value(2.into()).unwrap(), RawValue::U32(0x515253));
            assert_eq!(
                entry.get_value(3.into()).unwrap(),
                RawValue::Content(Content::new(
                    ContentAddress {
                        pack_id: PackId::from(0),
                        content_id: ContentIdx::from(0xaaaaaa)
                    },
                    None
                ))
            );
        }
    }
}
