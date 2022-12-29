use super::layout::Layout;
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
    fn produce(stream: &mut Stream) -> Result<Self> {
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

#[derive(Debug)]
pub enum EntryStore {
    Plain(PlainStore),
}

impl EntryStore {
    pub fn new(reader: &Reader, pos_info: SizedOffset) -> Result<Self> {
        let mut header_stream = reader.create_stream_for(pos_info);
        Ok(match StoreKind::produce(&mut header_stream)? {
            StoreKind::Plain => {
                EntryStore::Plain(PlainStore::new(&mut header_stream, reader, pos_info)?)
            }
            _ => todo!(),
        })
    }

    pub fn get_entry_reader(&self, idx: EntryIdx) -> Reader {
        match self {
            EntryStore::Plain(store) => store.get_entry_reader(idx),
            /*  todo!() */
        }
    }

    pub fn layout(&self) -> &Layout {
        match self {
            EntryStore::Plain(store) => store.layout(),
            /*            _ => todo!()*/
        }
    }
}

#[derive(Debug)]
pub struct PlainStore {
    pub layout: Layout,
    pub entry_reader: Reader,
}

impl PlainStore {
    pub fn new(stream: &mut Stream, reader: &Reader, pos_info: SizedOffset) -> Result<Self> {
        let layout = Layout::produce(stream)?;
        let data_size = Size::produce(stream)?;
        // [TODO] use a array_reader here
        let entry_reader =
            reader.create_sub_reader(pos_info.offset - data_size, End::Size(data_size));
        Ok(Self {
            layout,
            entry_reader,
        })
    }

    fn get_entry_reader(&self, idx: EntryIdx) -> Reader {
        self.entry_reader.create_sub_reader(
            Offset::from(self.layout.size.into_u64() * idx.into_u64()),
            End::Size(self.layout.size),
        )
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::directory_pack::layout::{Property, PropertyKind};

    #[test]
    fn test_1variant_allproperties() {
        #[rustfmt::skip]
        let content = vec![
            0x00, // kind
            0x05, 0x67,        //entry_size (1383)
            0x00,        // variant count
            0x14,        // property count (21)
            0b0000_0111, // padding (8)
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
        let size = Size::from(content.len());
        let reader = Reader::new(content, End::None);
        let store = EntryStore::new(&reader, SizedOffset::new(size, Offset::zero())).unwrap();
        let store = match store {
            EntryStore::Plain(s) => s,
        };
        assert!(store.layout.variant_part.is_none());
        let expected = [
            Property::new(8, PropertyKind::ContentAddress(0)),
            Property::new(12, PropertyKind::ContentAddress(1)),
            Property::new(20, PropertyKind::UnsignedInt(1)),
            Property::new(21, PropertyKind::UnsignedInt(3)),
            Property::new(24, PropertyKind::UnsignedInt(8)),
            Property::new(32, PropertyKind::SignedInt(1)),
            Property::new(33, PropertyKind::SignedInt(3)),
            Property::new(36, PropertyKind::SignedInt(8)),
            Property::new(44, PropertyKind::Array(1)),
            Property::new(45, PropertyKind::Array(8)),
            Property::new(53, PropertyKind::Array(9)),
            Property::new(62, PropertyKind::Array(264)),
            Property::new(326, PropertyKind::Array(1032)),
            Property::new(1358, PropertyKind::VLArray(1, 0x0F.into(), None)),
            Property::new(1359, PropertyKind::VLArray(8, 0x0F.into(), None)),
            Property::new(1367, PropertyKind::VLArray(1, 0x0F.into(), Some(2))),
            Property::new(1370, PropertyKind::VLArray(8, 0x0F.into(), Some(2))),
        ];
        assert_eq!(&*store.layout.common, &expected);
    }

    #[test]
    fn test_2variants() {
        #[rustfmt::skip]
        let content = vec![
            0x00, // kind
            0x00, 0x12,        //entry_size (18)
            0x02,        // variant count
            0x0A,        // property count (10)
            0b1000_0000, // Variant id
            0b0111_0100, 0x0F, // PstringLookup(5), idx 0x0F
            0b0100_0000,       // base char[1]
            0b0000_0011, // padding(4)
            0b0001_0000, // classic content address
            0b0010_0010, // u24
            0b1000_0000, // Variant id
            0b0100_0101, // char[6]
            0b0001_0001, // patch content address
            0b0010_0010, // u24
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //data size
        ];
        let size = Size::from(content.len());
        let reader = Reader::new(content, End::None);
        let store = EntryStore::new(&reader, SizedOffset::new(size, Offset::zero())).unwrap();
        let store = match store {
            EntryStore::Plain(s) => s,
        };
        let (offset, variants) = store.layout.variant_part.unwrap();
        assert_eq!(offset, Offset::zero());
        assert_eq!(variants.len(), 2);
        let variant = &variants[0];
        let expected = [
            Property::new(1, PropertyKind::VLArray(5, 0x0F.into(), Some(1))),
            Property::new(11, PropertyKind::ContentAddress(0)),
            Property::new(15, PropertyKind::UnsignedInt(3)),
        ];
        assert_eq!(&**variant, &expected);
        let variant = &variants[1];
        let expected = [
            Property::new(1, PropertyKind::Array(6)),
            Property::new(7, PropertyKind::ContentAddress(1)),
            Property::new(15, PropertyKind::UnsignedInt(3)),
        ];
        assert_eq!(&**variant, &expected);
    }
}
