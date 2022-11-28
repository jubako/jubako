use super::{EntryStore, Finder, Resolver};
use crate::bases::*;
use crate::common::ContentAddress;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq)]
pub struct IndexHeader {
    pub store_id: EntryStoreIdx,
    pub entry_count: EntryCount,
    pub entry_offset: EntryIdx,
    pub extra_data: ContentAddress,
    pub index_property: u8,
    pub name: String,
}

impl Producable for IndexHeader {
    type Output = Self;
    fn produce(stream: &mut Stream) -> Result<Self> {
        let store_id = Idx::<u32>::produce(stream)?.into();
        let entry_count = Count::<u32>::produce(stream)?.into();
        let entry_offset = Idx::<u32>::produce(stream)?.into();
        let extra_data = ContentAddress::produce(stream)?;
        let index_property = stream.read_u8()?;
        let name = String::from_utf8(PString::produce(stream)?)?;
        Ok(Self {
            store_id,
            entry_count,
            entry_offset,
            extra_data,
            index_property,
            name,
        })
    }
}

#[derive(Debug)]
pub struct Index {
    header: IndexHeader,
    store: Rc<EntryStore>,
}

impl Index {
    pub fn new(header: IndexHeader, store: Rc<EntryStore>) -> Self {
        Self { header, store }
    }

    fn entry_offset(&self) -> EntryIdx {
        self.header.entry_offset
    }

    pub fn entry_count(&self) -> EntryCount {
        self.header.entry_count
    }

    pub fn get_finder(&self, resolver: Rc<Resolver>) -> Finder {
        Finder::new(
            Rc::clone(&self.store),
            self.entry_offset(),
            self.entry_count(),
            resolver,
        )
    }

    pub fn get_store(&self) -> Rc<EntryStore> {
        Rc::clone(&self.store)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index() {
        let content = vec![
            0x00, 0x00, 0x00, 0x01, // store_id
            0x00, 0x00, 0xff, 0x00, // entry_count
            0x00, 0x00, 0x00, 0x02, // entry_offset
            0x05, 0x00, 0x00, 0x01, // extra_data
            0x01, // index_property
            0x05, 0x48, 0x65, 0x6C, 0x6C, 0x6F, // PString Hello
        ];
        let reader = Reader::new(content, End::None);
        let mut stream = reader.create_stream_all();
        let header = IndexHeader::produce(&mut stream).unwrap();
        assert_eq!(
            header,
            IndexHeader {
                store_id: EntryStoreIdx::from(1),
                entry_count: EntryCount::from(0xff00),
                entry_offset: EntryIdx::from(2),
                extra_data: ContentAddress {
                    pack_id: PackId::from(5),
                    content_id: ContentIdx::from(1)
                },
                index_property: 1,
                name: String::from("Hello")
            }
        );
    }
}
