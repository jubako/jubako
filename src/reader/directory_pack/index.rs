use super::{EntryRange, EntryStorage, EntryStore, RangeTrait};
use crate::bases::*;
use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename = "Index")]
pub struct IndexHeader {
    pub store_id: EntryStoreIdx,
    pub entry_count: EntryCount,
    pub entry_offset: EntryIdx,
    pub free_data: IndexFreeData,
    pub index_property: u8,
    pub name: String,
}

impl Producable for IndexHeader {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let store_id = Idx::<u32>::produce(flux)?.into();
        let entry_count = Count::<u32>::produce(flux)?.into();
        let entry_offset = Idx::<u32>::produce(flux)?.into();
        let free_data = IndexFreeData::produce(flux)?;
        let index_property = flux.read_u8()?;
        let name = String::from_utf8(PString::produce(flux)?)?;
        Ok(Self {
            store_id,
            entry_count,
            entry_offset,
            free_data,
            index_property,
            name,
        })
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(transparent)]
pub struct Index {
    header: IndexHeader,
}

impl Index {
    pub fn new(header: IndexHeader) -> Self {
        Self { header }
    }

    pub fn get_store(&self, entry_storage: &EntryStorage) -> Result<Arc<EntryStore>> {
        Ok(Arc::clone(
            entry_storage.get_entry_store(self.header.store_id)?,
        ))
    }

    pub fn get_store_id(&self) -> EntryStoreIdx {
        self.header.store_id
    }

    pub fn size(&self) -> EntryCount {
        self.header.entry_count
    }

    pub fn is_empty(&self) -> bool {
        self.header.entry_count.into_u32() == 0
    }
}

impl From<&Index> for EntryRange {
    fn from(index: &Index) -> Self {
        Self::new_from_size(index.offset(), index.count())
    }
}

impl RangeTrait for Index {
    fn offset(&self) -> EntryIdx {
        self.header.entry_offset
    }

    fn count(&self) -> EntryCount {
        self.header.entry_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index() {
        let content = vec![
            0x01, 0x00, 0x00, 0x00, // store_id
            0x00, 0xff, 0x00, 0x00, // entry_count
            0x02, 0x00, 0x00, 0x00, // entry_offset
            0x00, 0x00, 0x00, 0x00, // free data
            0x01, // index_property
            0x05, 0x48, 0x65, 0x6C, 0x6C, 0x6F, // PString Hello
        ];
        let reader = Reader::from(content);
        let mut flux = reader.create_flux_all();
        let header = IndexHeader::produce(&mut flux).unwrap();
        assert_eq!(
            header,
            IndexHeader {
                store_id: EntryStoreIdx::from(1),
                entry_count: EntryCount::from(0xff00),
                entry_offset: EntryIdx::from(2),
                free_data: [0x00; 4].into(),
                index_property: 1,
                name: String::from("Hello")
            }
        );
    }
}
