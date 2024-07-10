pub mod builder;
mod entry_store;
mod index;
pub mod layout;
mod lazy_entry;
mod property_compare;
mod range;
mod raw_layout;
mod raw_value;
mod value_store;

use self::index::IndexHeader;
use crate::bases::*;
use crate::common::{CheckInfo, DirectoryPackHeader, Pack, PackHeader, PackKind};
use std::sync::{Arc, RwLock};
use uuid::Uuid;

pub use self::entry_store::EntryStore;
pub use self::index::Index;
pub(self) use self::property_compare::PropertyCompare;
pub use self::range::{CompareTrait, RangeTrait};
pub(crate) use self::value_store::{ValueStore, ValueStoreTrait};
use crate::common::ContentAddress;
use crate::common::Value;
pub use lazy_entry::LazyEntry;
pub use raw_value::RawValue;
pub(crate) use raw_value::{Array, Extend};

pub trait EntryTrait {
    fn get_variant_id(&self) -> Result<Option<VariantIdx>>;
    fn get_value(&self, name: &str) -> Result<RawValue>;
}

mod private {
    use super::*;
    pub trait ValueStorageTrait {
        type ValueStore: ValueStoreTrait + 'static;
        fn get_value_store(&self, id: ValueStoreIdx) -> Result<Arc<Self::ValueStore>>;
    }
}

pub struct ValueStorage(VecCache<ValueStore, DirectoryPack>);

impl ValueStorage {
    fn new(source: Arc<DirectoryPack>) -> Self {
        Self(VecCache::new(source))
    }
}

impl private::ValueStorageTrait for ValueStorage {
    type ValueStore = ValueStore;

    fn get_value_store(&self, store_id: ValueStoreIdx) -> Result<Arc<Self::ValueStore>> {
        Ok(Arc::clone(self.0.get(store_id)?))
    }
}

pub struct EntryStorage(VecCache<EntryStore, DirectoryPack>);

impl EntryStorage {
    fn new(source: Arc<DirectoryPack>) -> Self {
        Self(VecCache::new(source))
    }

    pub fn get_entry_store(&self, store_id: EntryStoreIdx) -> Result<&Arc<EntryStore>> {
        self.0.get(store_id)
    }
}

pub struct DirectoryPack {
    pack_header: PackHeader,
    header: DirectoryPackHeader,
    value_stores_ptrs: ArrayReader<SizedOffset, u8>,
    entry_stores_ptrs: ArrayReader<SizedOffset, u32>,
    index_ptrs: ArrayReader<SizedOffset, u32>,
    reader: Reader,
    check_info: RwLock<Option<CheckInfo>>,
}

impl DirectoryPack {
    pub fn new(reader: Reader) -> Result<DirectoryPack> {
        let reader = reader.create_sub_memory_reader(Offset::zero(), reader.size())?;
        let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero())?;
        if pack_header.magic != PackKind::Directory {
            return Err(format_error!("Pack Magic is not DirectoryPack"));
        }

        let header =
            reader.parse_block_at::<DirectoryPackHeader>(Offset::from(PackHeader::BLOCK_SIZE))?;
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
            pack_header,
            header,
            value_stores_ptrs,
            entry_stores_ptrs,
            index_ptrs,
            reader,
            check_info: RwLock::new(None),
        })
    }
    pub fn get_free_data(&self) -> &[u8] {
        self.header.free_data.as_ref()
    }

    pub fn get_index(&self, index_id: IndexIdx) -> Result<Index> {
        let sized_offset = self.index_ptrs.index(*index_id)?;
        let index_header = self
            .reader
            .parse_block_in::<IndexHeader>(sized_offset.offset, sized_offset.size)?;
        let index = Index::new(index_header);
        Ok(index)
    }

    pub fn get_index_from_name(&self, index_name: &str) -> Result<Index> {
        for index_id in self.header.index_count {
            let sized_offset = self.index_ptrs.index(*index_id)?;
            let index_header = self
                .reader
                .parse_block_in::<IndexHeader>(sized_offset.offset, sized_offset.size)?;
            if index_header.name == index_name {
                let index = Index::new(index_header);
                return Ok(index);
            }
        }
        Err(format!("Cannot find index {index_name}").into())
    }

    pub fn create_value_storage(self: &Arc<Self>) -> Arc<ValueStorage> {
        Arc::new(ValueStorage::new(Arc::clone(self)))
    }

    pub fn create_entry_storage(self: &Arc<Self>) -> Arc<EntryStorage> {
        Arc::new(EntryStorage::new(Arc::clone(self)))
    }
}

impl CachableSource<ValueStore> for DirectoryPack {
    type Idx = ValueStoreIdx;
    fn get_len(&self) -> usize {
        self.header.value_store_count.into_usize()
    }

    fn get_value(&self, id: Self::Idx) -> Result<Arc<ValueStore>> {
        let sized_offset = self.value_stores_ptrs.index(*id)?;
        Ok(Arc::new(
            self.reader.parse_data_block::<ValueStore>(sized_offset)?,
        ))
    }
}

impl CachableSource<EntryStore> for DirectoryPack {
    type Idx = EntryStoreIdx;
    fn get_len(&self) -> usize {
        self.header.entry_store_count.into_usize()
    }

    fn get_value(&self, id: Self::Idx) -> Result<Arc<EntryStore>> {
        let sized_offset = self.entry_stores_ptrs.index(*id)?;
        Ok(Arc::new(
            self.reader.parse_data_block::<EntryStore>(sized_offset)?,
        ))
    }
}

impl Pack for DirectoryPack {
    fn kind(&self) -> PackKind {
        self.pack_header.magic
    }
    fn app_vendor_id(&self) -> VendorId {
        self.pack_header.app_vendor_id
    }
    fn version(&self) -> (u8, u8) {
        (
            self.pack_header.major_version,
            self.pack_header.minor_version,
        )
    }
    fn uuid(&self) -> Uuid {
        self.pack_header.uuid
    }
    fn size(&self) -> Size {
        self.pack_header.file_size
    }
    fn check(&self) -> Result<bool> {
        if self.check_info.read().unwrap().is_none() {
            let check_info = self.reader.parse_block_in::<CheckInfo>(
                self.pack_header.check_info_pos,
                self.pack_header.check_info_size(),
            )?;
            let mut s_check_info = self.check_info.write().unwrap();
            *s_check_info = Some(check_info);
        }
        let mut check_stream = self
            .reader
            .create_stream(Offset::zero(), Size::from(self.pack_header.check_info_pos));
        self.check_info
            .read()
            .unwrap()
            .unwrap()
            .check(&mut check_stream)
    }
}

#[cfg(feature = "explorable")]
impl serde::Serialize for DirectoryPack {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut cont = serializer.serialize_struct("DirectoryPack", 5)?;
        cont.serialize_field("uuid", &self.uuid())?;
        cont.serialize_field(
            "indexes",
            &self
                .header
                .index_count
                .into_iter()
                .map(|c| {
                    let sized_offset = self.index_ptrs.index(*c).unwrap();
                    let index_header = self
                        .reader
                        .parse_block_in::<IndexHeader>(sized_offset.offset, sized_offset.size)
                        .unwrap();
                    Index::new(index_header)
                })
                .collect::<Vec<_>>(),
        )?;
        cont.serialize_field(
            "entry_stores",
            &self
                .header
                .entry_store_count
                .into_iter()
                .map(|c| {
                    let sized_offset = self.entry_stores_ptrs.index(*c).unwrap();
                    self.reader
                        .parse_data_block::<EntryStore>(sized_offset)
                        .unwrap()
                })
                .collect::<Vec<_>>(),
        )?;
        cont.serialize_field(
            "value_stores",
            &self
                .header
                .value_store_count
                .into_iter()
                .map(|c| {
                    let sized_offset = self.value_stores_ptrs.index(*c).unwrap();
                    self.reader
                        .parse_data_block::<ValueStore>(sized_offset)
                        .unwrap()
                })
                .collect::<Vec<_>>(),
        )?;
        cont.serialize_field("free_data", &self.header.free_data)?;
        cont.end()
    }
}

#[cfg(feature = "explorable")]
impl Explorable for DirectoryPack {
    fn explore_one(&self, item: &str) -> Result<Option<Box<dyn Explorable>>> {
        if let Some(item) = item.strip_prefix("e.") {
            let index = if let Ok(index) = item.parse::<u32>() {
                EntryStoreIdx::from(index)
            } else {
                let index = self.get_index_from_name(item)?;
                index.get_store_id()
            };
            let sized_offset = self.entry_stores_ptrs.index(*index)?;
            Ok(Some(Box::new(
                self.reader.parse_data_block::<EntryStore>(sized_offset)?,
            )))
        } else if let Some(item) = item.strip_prefix("v.") {
            let index = item
                .parse::<u8>()
                .map_err(|e| Error::from(format!("{e}")))?;
            let sized_offset = self.value_stores_ptrs.index(index.into())?;
            Ok(Some(Box::new(
                self.reader.parse_data_block::<ValueStore>(sized_offset)?,
            )))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::raw_value::*;
    use super::*;

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
        // Pack header offset 0/0x00
        let mut content = vec![
            0x6a, 0x62, 0x6b, 0x64, // magic
            0x00, 0x00, 0x00, 0x01, // app_vendor_id
            0x00, // major_version
            0x02, // minor_version
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, // uuid
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // padding
            0xAE, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // file_size
            0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // check_info_pos
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // reserved
            0x00, 0x00, 0x00, 0x00, // reserved
        ];
        content.extend_from_slice(&[0x77, 0x41, 0x69, 0x29]); // CRC

        // Directory pack header offset 64/0x40
        content.extend_from_slice(&[
            0x3D, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // index_ptr_pos
            0x13, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // entry_store_ptr_pos
            0xA4, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // value_store_ptr_pos
            0x01, 0x00, 0x00, 0x00, // index count
            0x01, 0x00, 0x00, 0x00, // entry_store count
            0x01, //value_store count
        ]);
        content.extend_from_slice(&[0xff; 27]); // free data
        content.extend_from_slice(&[0xD9, 0xA4, 0x04, 0x38]); // CRC

        // Value Store data. Offset 128/0x80
        content.extend_from_slice(&[
            b'H', b'e', b'l', b'l', b'o', // value 0
            b'F', b'o', b'o', // value 1
            b'J', 0xc5, 0xab, b'b', b'a', b'k', b'o', // value 2
        ]);
        content.extend_from_slice(&[0x71, 0x51, 0xDF, 0x1D]); // CRC

        // Value store header. Offset 128 + 15 + 4 = 147/0x93
        content.extend_from_slice(&[
            0x01, // kind
            0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // value count
            0x01, // offset_size
            0x0f, // data_size
            0x05, // Offset of entry 1
            0x08, // Offset of entry 2
        ]);
        content.extend_from_slice(&[0x1E, 0x6E, 0xE7, 0xB7]); // CRC

        // Value store ptr. Offset 147 + 13 + 4 = 164/0xA4 (value_store_ptr_pos)
        content.extend_from_slice(&[
            13, 0x00, //size
            0x93, 0x00, 0x00, 0x00, 0x00, 0x00, // Offset the tailler (147/0x93)
        ]);
        content.extend_from_slice(&[0xE0, 0x14, 0x59, 0xCA]); // CRC

        // One variant, with on Char1[0], a Char1[2]+Deported(1), a u24 and a content address
        // Entry store data. Offset 164+8+4=176/0xB0
        #[rustfmt::skip]
        content.extend_from_slice(&[
            0x05, 0x00, 0x05, b'A', b'B', 0x01, 0x13, 0x12, 0x11, 0x00, 0x00, 0x00, 0x00, // Entry 0
            0x07, 0x02, 0x07, b'a', b'B', 0x00, 0x23, 0x22, 0x21, 0x01, 0x00, 0x00, 0x00, // Entry 1
            0x03, 0x01, 0x09, b'A', b'B', 0x02, 0x33, 0x32, 0x31, 0x00, 0x01, 0x00, 0x00, // Entry 2
            0x07, 0x02, 0x05, b'A', b'B', 0x01, 0x43, 0x42, 0x41, 0x00, 0x02, 0x00, 0x00, // Entry 3
            0x05, 0x00, 0x05, 0x00, 0x00, 0x01, 0x53, 0x52, 0x51, 0x00, 0xaa, 0xaa, 0xaa, // Entry 4
        ]);
        content.extend_from_slice(&[0x4C, 0x67, 0x87, 0x9B]); // CRC

        // Entry store header Offset 176 + (13*5) + 4 = 245/0xF5
        #[rustfmt::skip]
        content.extend_from_slice(&[
            0x00, // kind
            0x05, 0x00, 0x00, 0x00, // entry_count (5)
            0x00, // flag
            0x0D, 0x00, // entry size (13)
            0x00, // variant count
            0x04, // value count
            0b0101_0001, 0b001_00000, 0x00, 1, b'A', // Char1[0] + deported 1, idx 0x00
            0b0101_0001, 0b001_00010, 0x00, 1, b'B', // Char1[2] + deported 1, idx 0x00
            0b0010_0010, 1, b'C', // u24
            0b0001_0010, 1, b'D', // content address
        ]);
        content.extend_from_slice(&[0x49, 0x82, 0x74, 0xD6]); // CRC

        // Entry store array. Offset 245 + 26 + 4 = 275/0x113 (entry_store_ptr_pos)
        content.extend_from_slice(&[
            26, 0x00, // size
            0xF5, 0x00, 0x00, 0x00, 0x00, 0x00, // offset of the tailler (245)
        ]);
        content.extend_from_slice(&[0x03, 0x73, 0x94, 0x0B]); // CRC

        // Index. Offset 275+8+4=287/0x11F)
        content.extend_from_slice(&[
            0x00, 0x00, 0x00, 0x00, // store_id
            0x04, 0x00, 0x00, 0x00, // entry_count (use only 4 from the 5 available)
            0x01, 0x00, 0x00, 0x00, // entry offset (skip the first one)
            0x00, 0x00, 0x00, 0x00, // free_data
            0x00, // index_property (use the first pstring a binary search property
            0x08, b'm', b'y', b' ', b'i', b'n', b'd', b'e', b'x', // Pstring "my index"
        ]);
        content.extend_from_slice(&[0x86, 0x5C, 0x21, 0xDF]); // CRC

        // Add a index_ptr (offset 287+26+4=317/0x13D) (index_ptr_pos)
        content.extend_from_slice(&[
            26, 0x00, //size (26)
            0x1F, 0x01, 0x00, 0x00, 0x00, 0x00, // offset
        ]);
        content.extend_from_slice(&[0x27, 0x31, 0x53, 0x6F]); // CRC

        // Check Info. Offset 317 + 8 + 4 = 329/0x149 (check_info_pos)
        let hash = blake3::hash(&content);
        content.push(0x01); // check info
        content.extend(hash.as_bytes());
        content.extend_from_slice(&[0x72, 0x28, 0x30, 0x8F]); // CRC

        // Footer offset 329 + 33 + 4 = 366/0x16E
        let mut footer = [0; 64];
        footer.copy_from_slice(&content[..64]);
        footer.reverse();
        content.extend_from_slice(&footer);

        // File size 366 + 64 = 430/0x1AE (file_size)

        let directory_pack = Arc::new(DirectoryPack::new(content.into()).unwrap());
        assert!(directory_pack.check().unwrap());
        let index = directory_pack.get_index(0.into()).unwrap();
        let value_storage = directory_pack.create_value_storage();
        let entry_storage = directory_pack.create_entry_storage();
        let builder = builder::AnyBuilder::new(
            index.get_store(&entry_storage).unwrap(),
            value_storage.as_ref(),
        )
        .unwrap();
        assert_eq!(index.count(), 4.into());
        {
            let entry = index.get_entry(&builder, 0.into()).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            let value0 = entry.get_value("A").unwrap();
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
            let value1 = entry.get_value("B").unwrap();
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
            assert_eq!(entry.get_value("C").unwrap(), RawValue::U32(0x212223));
            assert_eq!(
                entry.get_value("D").unwrap(),
                RawValue::Content(ContentAddress {
                    pack_id: PackId::from(1),
                    content_id: ContentIdx::from(0)
                })
            );
        }
        {
            let entry = index.get_entry(&builder, 1.into()).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            let value0 = entry.get_value("A").unwrap();
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
            let value1 = entry.get_value("B").unwrap();
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
            assert_eq!(entry.get_value("C").unwrap(), RawValue::U32(0x313233));
            assert_eq!(
                entry.get_value("D").unwrap(),
                RawValue::Content(ContentAddress {
                    pack_id: PackId::from(0),
                    content_id: ContentIdx::from(1)
                })
            );
        }
        {
            let entry = index.get_entry(&builder, 2.into()).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            let value0 = entry.get_value("A").unwrap();
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
            let value1 = entry.get_value("B").unwrap();
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
            assert_eq!(entry.get_value("C").unwrap(), RawValue::U32(0x414243));
            assert_eq!(
                entry.get_value("D").unwrap(),
                RawValue::Content(ContentAddress {
                    pack_id: PackId::from(0),
                    content_id: ContentIdx::from(2)
                })
            );
        }
        {
            let entry = index.get_entry(&builder, 3.into()).unwrap();
            assert_eq!(entry.get_variant_id().unwrap(), None);
            let value0 = entry.get_value("A").unwrap();
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
            let value1 = entry.get_value("B").unwrap();
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
            assert_eq!(entry.get_value("C").unwrap(), RawValue::U32(0x515253));
            assert_eq!(
                entry.get_value("D").unwrap(),
                RawValue::Content(ContentAddress {
                    pack_id: PackId::from(0),
                    content_id: ContentIdx::from(0xaaaaaa)
                })
            );
        }
    }
}
