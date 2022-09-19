mod entry;
mod entry_def;
mod index;
mod index_store;
mod key;
mod key_def;
mod key_store;
mod value;

use self::index::{Index, IndexHeader};
use self::index_store::IndexStore;
use self::key_store::KeyStore;
use crate::bases::*;
use crate::common::{CheckInfo, DirectoryPackHeader, Pack, PackKind};
use std::cell::{Cell, OnceCell};
use std::io::Read;
use std::rc::Rc;
use uuid::Uuid;

pub use entry::Entry;
pub use key_def::{KeyDef, KeyDefKind};
pub use value::{Array, Content, Extend, Value};

pub struct DirectoryPack {
    header: DirectoryPackHeader,
    key_stores_ptrs: ArrayReader<SizedOffset, u8>,
    entry_stores_ptrs: ArrayReader<SizedOffset, u32>,
    index_ptrs: ArrayReader<SizedOffset, u32>,
    reader: Box<dyn Reader>,
    check_info: Cell<Option<CheckInfo>>,
}

impl DirectoryPack {
    pub fn new(reader: Box<dyn Reader>) -> Result<DirectoryPack> {
        let mut stream = reader.create_stream_all();
        let header = DirectoryPackHeader::produce(stream.as_mut())?;
        let key_stores_ptrs = ArrayReader::new_from_reader(
            reader.as_ref(),
            header.key_store_ptr_pos,
            header.key_store_count,
        );
        let entry_stores_ptrs = ArrayReader::new_from_reader(
            reader.as_ref(),
            header.entry_store_ptr_pos,
            header.entry_store_count,
        );
        let index_ptrs =
            ArrayReader::new_from_reader(reader.as_ref(), header.index_ptr_pos, header.index_count);
        Ok(DirectoryPack {
            header,
            key_stores_ptrs,
            entry_stores_ptrs,
            index_ptrs,
            reader,
            check_info: Cell::new(None),
        })
    }
    pub fn get_free_data(&self) -> &[u8] {
        self.header.free_data.as_ref()
    }

    pub fn get_index(&self, index_id: Idx<u32>) -> Result<Index> {
        let sized_offset = self.index_ptrs.index(index_id);
        let mut index_stream = self.reader.create_stream_for(sized_offset);
        let index_header = IndexHeader::produce(index_stream.as_mut())?;
        let store = self.get_store(index_header.store_id)?;
        let index = Index::new(index_header, Box::new(store));
        Ok(index)
    }

    fn get_store(&self, store_id: Idx<u32>) -> Result<IndexStore> {
        let sized_offset = self.entry_stores_ptrs.index(store_id);
        IndexStore::new(self.reader.as_ref(), sized_offset)
    }

    pub(self) fn get_key_store(&self, store_id: Idx<u8>) -> Result<KeyStore> {
        let sized_offset = self.key_stores_ptrs.index(store_id);
        KeyStore::new(self.reader.as_ref(), sized_offset)
    }

    pub(self) fn get_key_store_count(&self) -> Count<u8> {
        self.header.key_store_count
    }

    pub fn get_key_storage(self: &Rc<Self>) -> Rc<KeyStorage> {
        Rc::new(KeyStorage::new(Rc::clone(self)))
    }
}

pub struct KeyStorage {
    directory: Rc<DirectoryPack>,
    stores: Vec<OnceCell<KeyStore>>,
}

impl KeyStorage {
    pub fn new(directory: Rc<DirectoryPack>) -> KeyStorage {
        let mut stores = Vec::new();
        stores.resize_with(directory.get_key_store_count().0 as usize, Default::default);
        Self { directory, stores }
    }

    fn get_key_store(&self, id: Idx<u8>) -> Result<&KeyStore> {
        self.stores[id.0 as usize].get_or_try_init(|| self._get_key_store(id))
    }

    fn _get_key_store(&self, id: Idx<u8>) -> Result<KeyStore> {
        self.directory.get_key_store(id)
    }

    pub fn get_data(&self, extend: &Extend) -> Result<Vec<u8>> {
        let key_store = self.get_key_store(extend.store_id)?;
        key_store.get_data(extend.key_id.into())
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
            let check_info = CheckInfo::produce(checkinfo_stream.as_mut())?;
            self.check_info.set(Some(check_info));
        }
        let mut check_stream = self
            .reader
            .create_stream_to(End::Offset(self.header.pack_header.check_info_pos));
        self.check_info
            .get()
            .unwrap()
            .check(&mut check_stream.as_mut() as &mut dyn Read)
    }
}

#[cfg(test)]
mod tests {
    use super::value::*;
    use super::*;
    use crate::common::{ContentAddress, PackHeader};

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
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xee, 0xaa, // key_store_ptr_pos
            0x00, 0x00, 0x00, 0x50, // index count
            0x00, 0x00, 0x00, 0x60, // entry_store count
            0x05, //key_store count
        ];
        content.extend_from_slice(&[0xff; 31]);
        let reader = BufReader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        assert_eq!(
            DirectoryPackHeader::produce(stream.as_mut()).unwrap(),
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
                key_store_ptr_pos: Offset::from(0xeeaa_u64),
                index_count: Count::from(0x50_u32),
                entry_store_count: Count::from(0x60_u32),
                key_store_count: Count::from(0x05_u8),
                free_data: FreeData::clone_from_slice(&[0xff; 31]),
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
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x9C, // key_store_ptr_pos
            0x00, 0x00, 0x00, 0x01, // index count
            0x00, 0x00, 0x00, 0x01, // entry_store count
            0x01, //key_store count
        ];
        content.extend_from_slice(&[0xff; 31]); // free data
                                                // Add one key store offset 128/0x80
        content.extend_from_slice(&[
            b'H', b'e', b'l', b'l', b'o', // key 0
            b'F', b'o', b'o', // key 1
            b'J', 0xc5, 0xab, b'b', b'a', b'k', b'o', // key 2
            0x01, // kind
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // key count
            0x01, // offset_size
            0x0f, // data_size
            0x05, // Offset of entry 1
            0x08, // Offset of entry 2
        ]);
        // Add key_stores_ptr (offset 128+15+13=156/0x9C)
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
            0x05, // key count
            0b0110_0000, 0x00, // Pstring(1), idx 0x00
            0b0111_0000, 0x00,        // Psstringlookup(1), idx 0x00
            0b0100_0001, // char[2]
            0b0010_0010, // u24
            0b0001_0000, // content address
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x37, // data size
        ]);
        // Add a index_store_ptr (offset 164+55+20=239/0xEF)
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
            0x00, // index_key (use the first pstring a binary search key
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
        let reader = Box::new(BufReader::new(content, End::None));
        let directory_pack = Rc::new(DirectoryPack::new(reader).unwrap());
        let index = directory_pack.get_index(Idx(0)).unwrap();
        let key_storage = directory_pack.get_key_storage();
        assert_eq!(index.entry_count().0, 4);
        {
            let entry = index.get_entry(Idx(0)).unwrap();
            assert_eq!(entry.get_variant_id(), 0);
            let value0 = if let Value::Array(a) = entry.get_value(Idx(0)).unwrap() {
                a
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                value0,
                &Array::new(Vec::new(), Some(Extend::new(Idx(0), 2)))
            );
            assert_eq!(
                value0.resolve_to_vec(&key_storage).unwrap(),
                b"J\xc5\xabbako" // Jūbako
            );
            let value1 = if let Value::Array(a) = entry.get_value(Idx(1)).unwrap() {
                a
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                value1,
                &Array::new(vec![b'a', b'B'], Some(Extend::new(Idx(0), 0)))
            );
            assert_eq!(value1.resolve_to_vec(&key_storage).unwrap(), b"aBHello");
            assert_eq!(entry.get_value(Idx(2)).unwrap(), &Value::U32(0x212223));
            assert_eq!(
                entry.get_value(Idx(3)).unwrap(),
                &Value::Content(Content::new(
                    ContentAddress {
                        pack_id: Id(1),
                        content_id: Idx(0)
                    },
                    None
                ))
            );
        }
        {
            let entry = index.get_entry(Idx(1)).unwrap();
            assert_eq!(entry.get_variant_id(), 0);
            let value0 = if let Value::Array(a) = entry.get_value(Idx(0)).unwrap() {
                a
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                value0,
                &Array::new(Vec::new(), Some(Extend::new(Idx(0), 1)))
            );
            assert_eq!(value0.resolve_to_vec(&key_storage).unwrap(), b"Foo");
            let value1 = if let Value::Array(a) = entry.get_value(Idx(1)).unwrap() {
                a
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                value1,
                &Array::new(vec![b'A', b'B'], Some(Extend::new(Idx(0), 2)))
            );
            assert_eq!(
                value1.resolve_to_vec(&key_storage).unwrap(),
                b"ABJ\xc5\xabbako"
            );
            assert_eq!(entry.get_value(Idx(2)).unwrap(), &Value::U32(0x313233));
            assert_eq!(
                entry.get_value(Idx(3)).unwrap(),
                &Value::Content(Content::new(
                    ContentAddress {
                        pack_id: Id(0),
                        content_id: Idx(1)
                    },
                    None
                ))
            );
        }
        {
            let entry = index.get_entry(Idx(2)).unwrap();
            assert_eq!(entry.get_variant_id(), 0);
            let value0 = if let Value::Array(a) = entry.get_value(Idx(0)).unwrap() {
                a
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                value0,
                &Array::new(Vec::new(), Some(Extend::new(Idx(0), 2)))
            );
            assert_eq!(
                value0.resolve_to_vec(&key_storage).unwrap(),
                b"J\xc5\xabbako"
            );
            let value1 = if let Value::Array(a) = entry.get_value(Idx(1)).unwrap() {
                a
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                value1,
                &Array::new(vec![b'A', b'B'], Some(Extend::new(Idx(0), 1)))
            );
            assert_eq!(value1.resolve_to_vec(&key_storage).unwrap(), b"ABFoo");
            assert_eq!(entry.get_value(Idx(2)).unwrap(), &Value::U32(0x414243));
            assert_eq!(
                entry.get_value(Idx(3)).unwrap(),
                &Value::Content(Content::new(
                    ContentAddress {
                        pack_id: Id(0),
                        content_id: Idx(2)
                    },
                    None
                ))
            );
        }
        {
            let entry = index.get_entry(Idx(3)).unwrap();
            assert_eq!(entry.get_variant_id(), 0);
            let value0 = if let Value::Array(a) = entry.get_value(Idx(0)).unwrap() {
                a
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                value0,
                &Array::new(Vec::new(), Some(Extend::new(Idx(0), 0)))
            );
            assert_eq!(value0.resolve_to_vec(&key_storage).unwrap(), b"Hello");
            let value1 = if let Value::Array(a) = entry.get_value(Idx(1)).unwrap() {
                a
            } else {
                panic!("Must be a array");
            };
            assert_eq!(
                value1,
                &Array::new(vec![0, 0], Some(Extend::new(Idx(0), 1)))
            );
            assert_eq!(value1.resolve_to_vec(&key_storage).unwrap(), b"\0\0Foo");
            assert_eq!(entry.get_value(Idx(2)).unwrap(), &Value::U32(0x515253));
            assert_eq!(
                entry.get_value(Idx(3)).unwrap(),
                &Value::Content(Content::new(
                    ContentAddress {
                        pack_id: Id(0),
                        content_id: Idx(0xaaaaaa)
                    },
                    None
                ))
            );
        }
    }
}
