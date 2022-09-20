use super::index_store::IndexStore;
use super::lazy_entry::LazyEntry;
use crate::bases::*;
use crate::common::ContentAddress;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq)]
pub struct IndexHeader {
    pub store_id: Idx<u32>,
    pub entry_count: Count<u32>,
    pub entry_offset: Idx<u32>,
    pub extra_data: ContentAddress,
    pub index_key: u8,
    pub name: String,
}

impl Producable for IndexHeader {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let store_id = Idx::<u32>::produce(stream)?;
        let entry_count = Count::<u32>::produce(stream)?;
        let entry_offset = Idx::<u32>::produce(stream)?;
        let extra_data = ContentAddress::produce(stream)?;
        let index_key = stream.read_u8()?;
        let name = String::from_utf8(PString::produce(stream)?)?;
        Ok(Self {
            store_id,
            entry_count,
            entry_offset,
            extra_data,
            index_key,
            name,
        })
    }
}

#[derive(Debug)]
pub struct Index {
    header: IndexHeader,
    store: Rc<IndexStore>,
}

impl Index {
    pub fn new(header: IndexHeader, store: Rc<IndexStore>) -> Self {
        Self { header, store }
    }

    pub fn entry_offset(&self) -> Idx<u32> {
        self.header.entry_offset
    }

    pub fn entry_count(&self) -> Count<u32> {
        self.header.entry_count
    }

    pub fn get_entry(&self, id: Idx<u32>) -> Result<LazyEntry> {
        if id.is_valid(self.entry_count()) {
            let id = self.header.entry_offset + id;
            self.store.get_entry(id)
        } else {
            Err("Invalid id".to_string().into())
        }
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
            0x01, // index_key
            0x05, 0x48, 0x65, 0x6C, 0x6C, 0x6F, // PString Hello
        ];
        let reader = Box::new(BufReader::new(content, End::None));
        let mut stream = reader.create_stream_all();
        let header = IndexHeader::produce(stream.as_mut()).unwrap();
        assert_eq!(
            header,
            IndexHeader {
                store_id: Idx(1),
                entry_count: Count(0xff00),
                entry_offset: Idx(2),
                extra_data: ContentAddress {
                    pack_id: Id(5),
                    content_id: Idx(1)
                },
                index_key: 1,
                name: String::from("Hello")
            }
        );
    }
}
