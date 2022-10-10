use super::entry::EntryTrait;
use super::entry_def::EntryDef;
use super::lazy_entry::LazyEntry;
use crate::bases::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum StoreKind {
    Plain = 0,
    Ref = 1,
    Full = 2,
}

impl Producable for StoreKind {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        match stream.read_u8()? {
            0 => Ok(StoreKind::Plain),
            1 => Ok(StoreKind::Ref),
            2 => Ok(StoreKind::Full),
            v => Err(format_error!(
                &format!("Invalid store kind ({})", v),
                stream
            )),
        }
    }
}

pub trait IndexStoreTrait {
    type Entry: EntryTrait;
    fn get_entry(&self, idx: Idx<u32>) -> Result<Self::Entry>;
}

#[derive(Debug)]
pub enum IndexStore {
    Plain(PlainStore),
}

impl IndexStore {
    pub fn new(reader: &dyn Reader, pos_info: SizedOffset) -> Result<Self> {
        let mut header_stream = reader.create_stream_for(pos_info);
        Ok(match StoreKind::produce(header_stream.as_mut())? {
            StoreKind::Plain => {
                IndexStore::Plain(PlainStore::new(header_stream.as_mut(), reader, pos_info)?)
            }
            _ => todo!(),
        })
    }
}

impl IndexStoreTrait for IndexStore {
    type Entry = LazyEntry;
    fn get_entry(&self, idx: Idx<u32>) -> Result<LazyEntry> {
        match self {
            IndexStore::Plain(store) => store.get_entry(idx),
            /*            _ => todo!()*/
        }
    }
}

#[derive(Debug)]
pub struct PlainStore {
    pub entry_def: EntryDef,
    pub entry_reader: Box<dyn Reader>,
}

impl PlainStore {
    pub fn new(
        stream: &mut dyn Stream,
        reader: &dyn Reader,
        pos_info: SizedOffset,
    ) -> Result<Self> {
        let entry_def = EntryDef::produce(stream)?;
        let data_size = Size::produce(stream)?;
        // [TODO] use a array_reader here
        let entry_reader = reader.create_sub_reader(
            Offset(pos_info.offset.0 - data_size.0),
            End::Size(data_size),
        );
        Ok(Self {
            entry_def,
            entry_reader,
        })
    }

    pub fn get_entry(&self, idx: Idx<u32>) -> Result<LazyEntry> {
        let reader = self.entry_reader.create_sub_memory_reader(
            Offset(idx.0 as u64 * self.entry_def.size.0),
            End::Size(self.entry_def.size),
        )?;
        self.entry_def.create_entry(reader.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::super::key::{Key, KeyKind};
    use super::*;

    #[test]
    fn test_1variant_allkeys() {
        #[rustfmt::skip]
        let content = vec![
            0x00, // kind
            0x05, 0x67,        //entry_size (1383)
            0x01,        // variant count
            0x15,        // key count (21)
            0b1000_0000, // Variant id
            0b0000_0111, // padding key(8)
            0b0001_0000, // classic content address
            0b0001_0001, // patch content address
            0b0010_0000, // u8
            0b0010_0010, // u24
            0b0010_0111, // u64
            0b0010_1000, // s8
            0b0010_1010, // s24
            0b0010_1111, // s64
            0b0100_0000, // char[1]
            0b0100_0111, // char[8]
            0b0100_1000, 0x00, // char[9]
            0b0100_1000, 0xFF, // char[264] (255+9)
            0b0100_1011, 0xFF, // char[1032] (1023+9)
            0b0110_0000, 0x0F, // Pstring(1), idx 0x0F
            0b0110_0111, 0x0F, // Pstring(8), idx 0x0F
            0b0111_0000, 0x0F, // PstringLookup(1), idx 0x0F
            0b0100_0001, // base char[2]
            0b0111_0111, 0x0F, // PstringLookup(8), idx 0x0F
            0b0100_0001, // base char[2]
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //data size
        ];
        let size = Size(content.len() as u64);
        let reader = Box::new(BufReader::new(content, End::None));
        let store = IndexStore::new(reader.as_ref(), SizedOffset::new(size, Offset(0))).unwrap();
        let store = match store {
            IndexStore::Plain(s) => s,
        };
        assert_eq!(store.entry_def.variants.len(), 1);
        let variant = &store.entry_def.variants[0];
        let expected = vec![
            Key::new(9, KeyKind::ContentAddress(0)),
            Key::new(13, KeyKind::ContentAddress(1)),
            Key::new(21, KeyKind::UnsignedInt(1)),
            Key::new(22, KeyKind::UnsignedInt(3)),
            Key::new(25, KeyKind::UnsignedInt(8)),
            Key::new(33, KeyKind::SignedInt(1)),
            Key::new(34, KeyKind::SignedInt(3)),
            Key::new(37, KeyKind::SignedInt(8)),
            Key::new(45, KeyKind::CharArray(1)),
            Key::new(46, KeyKind::CharArray(8)),
            Key::new(54, KeyKind::CharArray(9)),
            Key::new(63, KeyKind::CharArray(264)),
            Key::new(327, KeyKind::CharArray(1032)),
            Key::new(1359, KeyKind::PString(1, 0x0F.into(), None)),
            Key::new(1360, KeyKind::PString(8, 0x0F.into(), None)),
            Key::new(1368, KeyKind::PString(1, 0x0F.into(), Some(2))),
            Key::new(1371, KeyKind::PString(8, 0x0F.into(), Some(2))),
        ];
        assert_eq!(&variant.keys, &expected);
    }

    #[test]
    fn test_2variants() {
        #[rustfmt::skip]
        let content = vec![
            0x00, // kind
            0x00, 0x12,        //entry_size (18)
            0x02,        // variant count
            0x0A,        // key count (10)
            0b1000_0000, // Variant id
            0b0111_0100, 0x0F, // PstringLookup(5), idx 0x0F
            0b0100_0000,       // base char[1]
            0b0000_0011, // padding key(4)
            0b0001_0000, // classic content address
            0b0010_0010, // u24
            0b1000_0000, // Variant id
            0b0100_0101, // char[6]
            0b0001_0001, // patch content address
            0b0010_0010, // u24
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //data size
        ];
        let size = Size(content.len() as u64);
        let reader = Box::new(BufReader::new(content, End::None));
        let store = IndexStore::new(reader.as_ref(), SizedOffset::new(size, Offset(0))).unwrap();
        let store = match store {
            IndexStore::Plain(s) => s,
        };
        assert_eq!(store.entry_def.variants.len(), 2);
        let variant = &store.entry_def.variants[0];
        let expected = vec![
            Key::new(1, KeyKind::PString(5, 0x0F.into(), Some(1))),
            Key::new(11, KeyKind::ContentAddress(0)),
            Key::new(15, KeyKind::UnsignedInt(3)),
        ];
        assert_eq!(&variant.keys, &expected);
        let variant = &store.entry_def.variants[1];
        let expected = vec![
            Key::new(1, KeyKind::CharArray(6)),
            Key::new(7, KeyKind::ContentAddress(1)),
            Key::new(15, KeyKind::UnsignedInt(3)),
        ];
        assert_eq!(&variant.keys, &expected);
    }
}
