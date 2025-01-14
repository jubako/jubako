use super::layout::Layout;
use crate::{bases::*, reader::ByteSlice};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum StoreKind {
    Plain = 0,
    Ref = 1,
    Full = 2,
}

impl Parsable for StoreKind {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        match parser.read_u8()? {
            0 => Ok(StoreKind::Plain),
            1 => Ok(StoreKind::Ref),
            2 => Ok(StoreKind::Full),
            v => Err(format_error!(&format!("Invalid store kind ({v})"), parser)),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "explorable_serde", derive(serde::Serialize))]
pub enum EntryStore {
    Plain(PlainStore),
}

impl EntryStore {
    pub fn get_entry_reader(&self, idx: EntryIdx) -> Option<ByteSlice> {
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

pub(crate) struct EntryStoreBuilder {}

impl Parsable for EntryStoreBuilder {
    type Output = Layout;
    fn parse(parser: &mut impl Parser) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        let kind = StoreKind::parse(parser)?;
        match kind {
            StoreKind::Plain => {
                let layout = Layout::parse(parser)?;
                Ok(layout)
            }
            _ => todo!(),
        }
    }
}

impl BlockParsable for EntryStoreBuilder {}

impl DataBlockParsable for EntryStore {
    type TailParser = EntryStoreBuilder;
    type Output = Self;

    fn finalize(layout: Layout, header_offset: Offset, reader: &Reader) -> Result<Self> {
        let entry_reader = if layout.is_entry_checked {
            let data_size =
                layout.entry_count * (layout.entry_size + BlockCheck::Crc32.size()).into();
            reader.cut(header_offset - data_size, data_size, false)?
        } else {
            let data_size = layout.entry_count * layout.entry_size.into();
            reader
                .cut_check(
                    header_offset - data_size - ASize::from(BlockCheck::Crc32.size()),
                    data_size,
                    BlockCheck::Crc32,
                )?
                .into()
        };
        Ok(EntryStore::Plain(PlainStore {
            layout,
            entry_reader,
        }))
    }
}

#[cfg(feature = "explorable")]
impl graphex::Node for EntryStore {
    fn next(&self, key: &str) -> graphex::ExploreResult {
        match self {
            EntryStore::Plain(store) => store.next(key),
        }
    }

    fn display(&self) -> &dyn graphex::Display {
        self
    }

    #[cfg(feature = "explorable_serde")]
    fn serde(&self) -> Option<&dyn erased_serde::Serialize> {
        Some(self)
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for EntryStore {
    fn header_footer(&self) -> Option<(String, String)> {
        match self {
            EntryStore::Plain(store) => store.header_footer(),
        }
    }

    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        match self {
            EntryStore::Plain(store) => store.print_content(out),
        }
    }
}

#[derive(Debug)]
pub struct PlainStore {
    pub(crate) layout: Layout,
    pub(crate) entry_reader: Reader,
}

impl PlainStore {
    fn get_entry_reader(&self, idx: EntryIdx) -> Option<ByteSlice> {
        if idx.is_valid(*self.layout.entry_count) {
            Some(self.entry_reader.get_byte_slice(
                Offset::from(self.layout.entry_size.into_u64() * idx.into_u64()),
                self.layout.entry_size.into(),
            ))
        } else {
            None
        }
    }

    pub(crate) fn layout(&self) -> &Layout {
        &self.layout
    }
}

#[cfg(feature = "explorable_serde")]
impl serde::Serialize for PlainStore {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut ser = serializer.serialize_struct("PlainStore", 2)?;
        ser.serialize_field("entries count", &self.layout.entry_count)?;
        ser.serialize_field("layout", &self.layout)?;
        ser.end()
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for PlainStore {
    fn header_footer(&self) -> Option<(String, String)> {
        Some(("PlainStore(".to_string(), ")".to_string()))
    }
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        use yansi::Paint;
        out.field(&format!("entries count ({})", "<N>".bold()), &self.layout)?;
        out.field("layout", &self.layout)
    }
}

#[cfg(feature = "explorable")]
impl graphex::Node for PlainStore {
    fn next(&self, key: &str) -> graphex::ExploreResult {
        use std::io::Read;
        let index = key
            .parse::<u32>()
            .map_err(|e| graphex::Error::key(&format!("{e}")))?;
        let entry_reader = self
            .get_entry_reader(EntryIdx::from(index))
            .ok_or_else(|| graphex::Error::key(&format!("Key {index} is not found in store")))?;
        let mut data = vec![];
        entry_reader
            .stream()
            .read_to_end(&mut data)
            .map_err(|e| graphex::Error::from(Error::from(e)))?;
        Ok(Box::new(data).into())
    }

    fn display(&self) -> &dyn graphex::Display {
        &self.layout
    }

    #[cfg(feature = "explorable_serde")]
    fn serde(&self) -> Option<&dyn erased_serde::Serialize> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::{
        directory_pack::layout::{Property, PropertyKind},
        directory_pack::raw_layout::DeportedInfo,
        layout::VariantPart,
    };
    use std::collections::HashMap;

    #[test]
    fn test_1variant_allproperties() {
        #[rustfmt::skip]
        let content = vec![
            0xFF, 0xFF, 0xFF, 0xFF, // Dummy CRC of the non existant data
            0x00, // kind
            0x00, 0x00, 0x00, 0x00, // entry_count (0 as we are testing layout parsing only)
            0x00, // flag
            0x6D, 0x00,  //entry_size (108)
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
            0b0001_1100, 0x01, 0x02, 3 , b'V', b'1', b'4', // content address, with default 0x0201 and 1 byte of data offset: 108
            0x1A, 0x01, 0x65, 0xB7, // crc
        ];
        let size = ASize::from(content.len() - 8);
        let reader = Reader::from(content);
        let store = reader
            .parse_data_block::<EntryStore>(SizedOffset::new(size, Offset::new(4)))
            .unwrap();
        let EntryStore::Plain(store) = store;
        assert!(store.layout.variant_part.is_none());
        let expected = HashMap::from([
            (
                "V0".to_string(),
                Property::new(
                    8,
                    PropertyKind::ContentAddress {
                        pack_id_size: ByteSize::U1,
                        content_id_size: ByteSize::U3,
                        default_pack_id: None,
                    },
                ),
            ),
            (
                "V1".to_string(),
                Property::new(
                    12,
                    PropertyKind::UnsignedInt {
                        int_size: ByteSize::U1,
                        default: None,
                    },
                ),
            ),
            (
                "V2".to_string(),
                Property::new(
                    13,
                    PropertyKind::UnsignedInt {
                        int_size: ByteSize::U3,
                        default: None,
                    },
                ),
            ),
            (
                "V3".to_string(),
                Property::new(
                    16,
                    PropertyKind::UnsignedInt {
                        int_size: ByteSize::U8,
                        default: None,
                    },
                ),
            ),
            (
                "V4".to_string(),
                Property::new(
                    24,
                    PropertyKind::SignedInt {
                        int_size: ByteSize::U1,
                        default: None,
                    },
                ),
            ),
            (
                "V5".to_string(),
                Property::new(
                    25,
                    PropertyKind::SignedInt {
                        int_size: ByteSize::U3,
                        default: None,
                    },
                ),
            ),
            (
                "V6".to_string(),
                Property::new(
                    28,
                    PropertyKind::SignedInt {
                        int_size: ByteSize::U8,
                        default: None,
                    },
                ),
            ),
            (
                "V7".to_string(),
                Property::new(
                    36,
                    PropertyKind::Array {
                        array_len_size: Some(ByteSize::U2),
                        fixed_array_len: 1,
                        deported_info: None,
                        default: None,
                    },
                ),
            ),
            (
                "V8".to_string(),
                Property::new(
                    39,
                    PropertyKind::Array {
                        array_len_size: Some(ByteSize::U2),
                        fixed_array_len: 8,
                        deported_info: None,
                        default: None,
                    },
                ),
            ),
            (
                "V9".to_string(),
                Property::new(
                    49,
                    PropertyKind::Array {
                        array_len_size: Some(ByteSize::U2),
                        fixed_array_len: 31,
                        deported_info: None,
                        default: None,
                    },
                ),
            ),
            (
                "V10".to_string(),
                Property::new(
                    82,
                    PropertyKind::Array {
                        array_len_size: Some(ByteSize::U1),
                        fixed_array_len: 0,
                        deported_info: Some(DeportedInfo {
                            id_size: ByteSize::U1,
                            value_store_idx: 0x0F.into(),
                        }),
                        default: None,
                    },
                ),
            ),
            (
                "V11".to_string(),
                Property::new(
                    84,
                    PropertyKind::Array {
                        array_len_size: Some(ByteSize::U2),
                        fixed_array_len: 0,
                        deported_info: Some(DeportedInfo {
                            id_size: ByteSize::U7,
                            value_store_idx: 0x0F.into(),
                        }),
                        default: None,
                    },
                ),
            ),
            (
                "V12".to_string(),
                Property::new(
                    93,
                    PropertyKind::Array {
                        array_len_size: Some(ByteSize::U1),
                        fixed_array_len: 2,
                        deported_info: Some(DeportedInfo {
                            id_size: ByteSize::U1,
                            value_store_idx: 0x0F.into(),
                        }),
                        default: None,
                    },
                ),
            ),
            (
                "V13".to_string(),
                Property::new(
                    97,
                    PropertyKind::Array {
                        array_len_size: Some(ByteSize::U2),
                        fixed_array_len: 2,
                        deported_info: Some(DeportedInfo {
                            id_size: ByteSize::U7,
                            value_store_idx: 0x0F.into(),
                        }),
                        default: None,
                    },
                ),
            ),
            (
                "V14".to_string(),
                Property::new(
                    108,
                    PropertyKind::ContentAddress {
                        pack_id_size: ByteSize::U2,
                        content_id_size: ByteSize::U1,
                        default_pack_id: Some(0x0201.into()),
                    },
                ),
            ),
        ]);
        assert_eq!(&*store.layout.common, &expected);
    }

    #[test]
    fn test_2variants() {
        #[rustfmt::skip]
        let content = vec![
            0xFF, 0xFF, 0xFF, 0xFF, // Dummy CRC of the non existant data
            0x00, // kind
            0x00, 0x00, 0x00, 0x00, // entry_count (0),
            0x00, //flag
            0x1D, 0x00,  //entry_size (29)
            0x02,        // variant count
            0x0B,        // property count (9)
            0b0000_0110, // padding (7)       offset: 0
            0b0101_0011, 0b001_00001, 0x0F, 2, b'C', b'0', // char3[1] + deported(1) 0x0F                offset: 7
            0b1000_0000, 3, b'V', b'A', b'0', // Variant id size:1                                       offset: 12
            0b0101_0011, 0b101_00001, 0x0F, 2, b'V', b'0',  // char3[1] + deported(5), idx 0x0F size: 10 offset: 13
            0b0001_0010, 2, b'V', b'1', // content address size : 1+ 3                                   offset: 22
            0b0010_0010, 2, b'V', b'2', // u24 size: 3                                                   offset: 26  => Variant size 29
            0b1000_0000, 3, b'V', b'A', b'1', // Variant id size: 1                                      offset: 12  // new variant
            0b0101_0011, 0b000_00101, 2, b'V', b'0', // char3[6] size: 9                                 offset: 13
            0b0001_0101, 2, b'V', b'1',  // content address size: 2 + 2                                  offset: 21
            0b0010_0010, 2, b'V', b'2',  // u24 size: 3                                                  offset: 25
            0b0000_0000,  // padding (1)                                                                 offset: 28  => Variant size 29
            0x91, 0xFF, 0xB6, 0x2A, // crc
        ];
        let size = ASize::from(content.len() - 8);
        let reader = Reader::from(content);
        let store = reader
            .parse_data_block::<EntryStore>(SizedOffset::new(size, Offset::new(4)))
            .unwrap();
        let EntryStore::Plain(store) = store;
        let common = store.layout.common;
        let expected = HashMap::from([(
            "C0".to_string(),
            Property::new(
                7,
                PropertyKind::Array {
                    array_len_size: Some(ByteSize::U3),
                    fixed_array_len: 1,
                    deported_info: Some(DeportedInfo {
                        id_size: ByteSize::U1,
                        value_store_idx: 0x0F.into(),
                    }),
                    default: None,
                },
            ),
        )]);
        assert_eq!(&*common, &expected);

        let VariantPart {
            variant_id_offset,
            variants,
            names,
        } = store.layout.variant_part.unwrap();
        assert_eq!(variant_id_offset, Offset::new(12));
        assert_eq!(variants.len(), 2);
        assert_eq!(
            names,
            HashMap::from([(String::from("VA0"), 0), (String::from("VA1"), 1)])
        );
        let variant = &variants[0];
        let expected = HashMap::from([
            (
                "V0".to_string(),
                Property::new(
                    13,
                    PropertyKind::Array {
                        array_len_size: Some(ByteSize::U3),
                        fixed_array_len: 1,
                        deported_info: Some(DeportedInfo {
                            id_size: ByteSize::U5,
                            value_store_idx: 0x0F.into(),
                        }),
                        default: None,
                    },
                ),
            ),
            (
                "V1".to_string(),
                Property::new(
                    22,
                    PropertyKind::ContentAddress {
                        pack_id_size: ByteSize::U1,
                        content_id_size: ByteSize::U3,
                        default_pack_id: None,
                    },
                ),
            ),
            (
                "V2".to_string(),
                Property::new(
                    26,
                    PropertyKind::UnsignedInt {
                        int_size: ByteSize::U3,
                        default: None,
                    },
                ),
            ),
        ]);
        assert_eq!(***variant, expected);
        let variant = &variants[1];
        let expected = HashMap::from([
            (
                "V0".to_string(),
                Property::new(
                    13,
                    PropertyKind::Array {
                        array_len_size: Some(ByteSize::U3),
                        fixed_array_len: 5,
                        deported_info: None,
                        default: None,
                    },
                ),
            ),
            (
                "V1".to_string(),
                Property::new(
                    21,
                    PropertyKind::ContentAddress {
                        pack_id_size: ByteSize::U2,
                        content_id_size: ByteSize::U2,
                        default_pack_id: None,
                    },
                ),
            ),
            (
                "V2".to_string(),
                Property::new(
                    25,
                    PropertyKind::UnsignedInt {
                        int_size: ByteSize::U3,
                        default: None,
                    },
                ),
            ),
        ]);
        assert_eq!(***variant, expected);
    }
}
