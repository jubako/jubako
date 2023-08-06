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
    fn produce(flux: &mut Flux) -> Result<Self> {
        match flux.read_u8()? {
            0 => Ok(StoreKind::Plain),
            1 => Ok(StoreKind::Ref),
            2 => Ok(StoreKind::Full),
            v => Err(format_error!(&format!("Invalid store kind ({v})"), flux)),
        }
    }
}

#[derive(Debug)]
pub enum EntryStore {
    Plain(PlainStore),
}

impl EntryStore {
    pub fn new(reader: &Reader, pos_info: SizedOffset) -> Result<Self> {
        let mut header_flux = reader.create_flux_for(pos_info);
        Ok(match StoreKind::produce(&mut header_flux)? {
            StoreKind::Plain => {
                EntryStore::Plain(PlainStore::new(&mut header_flux, reader, pos_info)?)
            }
            _ => todo!(),
        })
    }

    pub fn get_entry_reader(&self, idx: EntryIdx) -> SubReader {
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
    pub fn new(flux: &mut Flux, reader: &Reader, pos_info: SizedOffset) -> Result<Self> {
        let layout = Layout::produce(flux)?;
        let data_size = Size::produce(flux)?;
        // [TODO] use a array_reader here
        let entry_reader = reader
            .create_sub_reader(pos_info.offset - data_size, End::Size(data_size))
            .into();
        Ok(Self {
            layout,
            entry_reader,
        })
    }

    fn get_entry_reader(&self, idx: EntryIdx) -> SubReader {
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
    use std::collections::HashMap;

    #[test]
    fn test_1variant_allproperties() {
        #[rustfmt::skip]
        let content = vec![
            0x00, // kind
            0x00, 0x6D,  //entry_size (108)
            0x00,        // variant count
            0x10,       // property count (16)
            0b0000_0111, // padding (8)       offset: 0
            0b0001_0010, 2, b'V', b'0', // content address   offset: 8
            0b0010_0000, 2, b'V', b'1', // u8                offset: 12
            0b0010_0010, 2, b'V', b'2', // u24               offset: 13
            0b0010_0111, 2, b'V', b'3', // u64               offset: 16
            0b0011_0000, 2, b'V', b'4', // s8                offset: 24
            0b0011_0010, 2, b'V', b'5', // s24               offset: 25
            0b0011_0111, 2, b'V', b'6', // s64               offset: 28
            0b0101_0010, 0b000_00001, 2, b'V', b'7', // char2[1]    offset: 36
            0b0101_0010, 0b000_01000, 2, b'V', b'8', // char2[8]    offset: 39
            0b0101_0010, 0b000_11111, 2, b'V', b'9', // char2[31]   offset: 49
            0b0101_0001, 0b001_00000, 0x0F, 3 , b'V', b'1', b'0', // char1[0] + deported(1), idx 0x0F   offset: 82
            0b0101_0010, 0b111_00000, 0x0F, 3 , b'V', b'1', b'1', // char2[0] + deported(7), idx 0x0F   offset: 84
            0b0101_0001, 0b001_00010, 0x0F, 3 , b'V', b'1', b'2', // char1[2] + deported(1), idx 0x0F   offset: 93
            0b0101_0010, 0b111_00010, 0x0F, 3 , b'V', b'1', b'3', // char2[2] + deported(7), idx 0x0F   offset: 97
            0b0001_1100, 0x02, 0x01, 3 , b'V', b'1', b'4', // content address, with default 0x0201 and 1 byte of data offset: 108
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //data size           offset: 109
        ];
        let size = Size::from(content.len());
        let reader = Reader::from(content);
        let store = EntryStore::new(&reader, SizedOffset::new(size, Offset::zero())).unwrap();
        let store = match store {
            EntryStore::Plain(s) => s,
        };
        assert!(store.layout.variant_part.is_none());
        let expected = HashMap::from([
            (
                "V0".to_string(),
                Property::new(
                    8,
                    PropertyKind::ContentAddress(ByteSize::U1, ByteSize::U3, None),
                ),
            ),
            (
                "V1".to_string(),
                Property::new(12, PropertyKind::UnsignedInt(ByteSize::U1, None)),
            ),
            (
                "V2".to_string(),
                Property::new(13, PropertyKind::UnsignedInt(ByteSize::U3, None)),
            ),
            (
                "V3".to_string(),
                Property::new(16, PropertyKind::UnsignedInt(ByteSize::U8, None)),
            ),
            (
                "V4".to_string(),
                Property::new(24, PropertyKind::SignedInt(ByteSize::U1, None)),
            ),
            (
                "V5".to_string(),
                Property::new(25, PropertyKind::SignedInt(ByteSize::U3, None)),
            ),
            (
                "V6".to_string(),
                Property::new(28, PropertyKind::SignedInt(ByteSize::U8, None)),
            ),
            (
                "V7".to_string(),
                Property::new(36, PropertyKind::Array(Some(ByteSize::U2), 1, None, None)),
            ),
            (
                "V8".to_string(),
                Property::new(39, PropertyKind::Array(Some(ByteSize::U2), 8, None, None)),
            ),
            (
                "V9".to_string(),
                Property::new(49, PropertyKind::Array(Some(ByteSize::U2), 31, None, None)),
            ),
            (
                "V10".to_string(),
                Property::new(
                    82,
                    PropertyKind::Array(
                        Some(ByteSize::U1),
                        0,
                        Some((ByteSize::U1, 0x0F.into())),
                        None,
                    ),
                ),
            ),
            (
                "V11".to_string(),
                Property::new(
                    84,
                    PropertyKind::Array(
                        Some(ByteSize::U2),
                        0,
                        Some((ByteSize::U7, 0x0F.into())),
                        None,
                    ),
                ),
            ),
            (
                "V12".to_string(),
                Property::new(
                    93,
                    PropertyKind::Array(
                        Some(ByteSize::U1),
                        2,
                        Some((ByteSize::U1, 0x0F.into())),
                        None,
                    ),
                ),
            ),
            (
                "V13".to_string(),
                Property::new(
                    97,
                    PropertyKind::Array(
                        Some(ByteSize::U2),
                        2,
                        Some((ByteSize::U7, 0x0F.into())),
                        None,
                    ),
                ),
            ),
            (
                "V14".to_string(),
                Property::new(
                    108,
                    PropertyKind::ContentAddress(ByteSize::U2, ByteSize::U1, Some(0x0201.into())),
                ),
            ),
        ]);
        assert_eq!(&*store.layout.common, &expected);
    }

    #[test]
    fn test_2variants() {
        #[rustfmt::skip]
        let content = vec![
            0x00, // kind
            0x00, 0x1F,  //entry_size (32)
            0x02,        // variant count
            0x0B,        // property count (9)
            0b0000_0110, // padding (7)       offset: 0
            0b0101_0100, 0b001_00001, 0x0F, 2, b'C', b'0', // char4[1] + deported(1) 0x0F                offset: 7
            0b1000_0000, 3, b'V', b'A', b'0', // Variant id size:1                                       offset: 13
            0b0101_0100, 0b101_00001, 0x0F, 2, b'V', b'0',  // char4[1] + deported(5), idx 0x0F size: 10 offset: 14
            0b0001_0010, 2, b'V', b'1', // content address size : 1+ 3                                   offset: 24
            0b0010_0010, 2, b'V', b'2', // u24 size: 3                                                   offset: 28  => Variant size 31
            0b1000_0000, 3, b'V', b'A', b'1', // Variant id size: 1                                      offset: 13  // new variant
            0b0101_0011, 0b000_00110, 2, b'V', b'0', // char3[6] size: 9                                 offset: 14
            0b0001_0101, 2, b'V', b'1',  // content address size: 2 + 2                                  offset: 23
            0b0010_0010, 2, b'V', b'2',  // u24 size: 3                                                  offset: 27
            0b0000_0000,  // padding (1)                                                                 offset: 30  => Variant size 31
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //data size
        ];
        let size = Size::from(content.len());
        let reader = Reader::from(content);
        let store = EntryStore::new(&reader, SizedOffset::new(size, Offset::zero())).unwrap();
        let store = match store {
            EntryStore::Plain(s) => s,
        };
        let common = store.layout.common;
        let expected = HashMap::from([(
            "C0".to_string(),
            Property::new(
                7,
                PropertyKind::Array(
                    Some(ByteSize::U4),
                    1,
                    Some((ByteSize::U1, 0x0F.into())),
                    None,
                ),
            ),
        )]);
        assert_eq!(&*common, &expected);

        let (offset, variants, variants_map) = store.layout.variant_part.unwrap();
        assert_eq!(offset, Offset::new(13));
        assert_eq!(variants.len(), 2);
        assert_eq!(
            variants_map,
            HashMap::from([(String::from("VA0"), 0), (String::from("VA1"), 1)])
        );
        let variant = &variants[0];
        let expected = HashMap::from([
            (
                "V0".to_string(),
                Property::new(
                    14,
                    PropertyKind::Array(
                        Some(ByteSize::U4),
                        1,
                        Some((ByteSize::U5, 0x0F.into())),
                        None,
                    ),
                ),
            ),
            (
                "V1".to_string(),
                Property::new(
                    24,
                    PropertyKind::ContentAddress(ByteSize::U1, ByteSize::U3, None),
                ),
            ),
            (
                "V2".to_string(),
                Property::new(28, PropertyKind::UnsignedInt(ByteSize::U3, None)),
            ),
        ]);
        assert_eq!(***variant, expected);
        let variant = &variants[1];
        let expected = HashMap::from([
            (
                "V0".to_string(),
                Property::new(14, PropertyKind::Array(Some(ByteSize::U3), 6, None, None)),
            ),
            (
                "V1".to_string(),
                Property::new(
                    23,
                    PropertyKind::ContentAddress(ByteSize::U2, ByteSize::U2, None),
                ),
            ),
            (
                "V2".to_string(),
                Property::new(27, PropertyKind::UnsignedInt(ByteSize::U3, None)),
            ),
        ]);
        assert_eq!(***variant, expected);
    }
}
