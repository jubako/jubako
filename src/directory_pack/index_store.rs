use super::entry::Entry;
use crate::bases::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StoreKind {
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
            _ => Err(Error::FormatError),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum KeyKind {
    Padding,
    ContentAddress { hidden: bool, patch: bool },
    UnsignedInt,
    SignedInt,
    CharArray,
    PString { keystoreidx: u8, flookup: bool },
    VariantId,
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct KeyDef {
    pub size: u16,
    pub kind: KeyKind,
}

impl Producable for KeyDef {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        let keyinfo = stream.read_u8()?;
        let keytype = keyinfo >> 4;
        let keydata = keyinfo & 0x0F;
        let (keysize, kind) = match keytype {
            0b0000 => (keydata as u16 + 1, KeyKind::Padding),
            0b0001 => match keydata {
                0b0000 => (
                    4,
                    KeyKind::ContentAddress {
                        hidden: false,
                        patch: false,
                    },
                ),
                0b0001 => (
                    4,
                    KeyKind::ContentAddress {
                        hidden: false,
                        patch: true,
                    },
                ),
                0b0010 => (
                    4,
                    KeyKind::ContentAddress {
                        hidden: true,
                        patch: false,
                    },
                ),
                0b0011 => (
                    4,
                    KeyKind::ContentAddress {
                        hidden: true,
                        patch: true,
                    },
                ),
                _ => return Err(Error::FormatError),
            },
            0b0010 => (
                (keydata & 0x07) as u16 + 1,
                if (keydata & 0x08) != 0 {
                    KeyKind::SignedInt
                } else {
                    KeyKind::UnsignedInt
                },
            ),
            0b0100 => {
                (
                    if keydata & 0x08 == 0 {
                        (keydata + 1) as u16
                    } else {
                        // We need a complement byte
                        let complement = stream.read_u8()?;
                        (((keydata & 0x03) as u16) << 8) + complement as u16 + 9
                    },
                    KeyKind::CharArray,
                )
            }
            0b0110 | 0b0111 => {
                let flookup: bool = keytype & 0b1 != 0;
                let size = keydata as u16 + 1 + flookup as u16;
                let keystoreidx = stream.read_u8()?;
                (
                    size,
                    KeyKind::PString {
                        keystoreidx,
                        flookup,
                    },
                )
            }
            0b1000 => (1, KeyKind::VariantId),
            _ => return Err(Error::FormatError),
        };
        Ok(Self {
            size: keysize,
            kind,
        })
    }
}

pub enum IndexStore {
    PLAIN(PlainStore),
}

impl IndexStore {
    pub fn new(reader: &dyn Reader, pos_info: SizedOffset) -> Result<Self> {
        let mut header_stream = reader.create_stream_for(pos_info);
        Ok(match StoreKind::produce(header_stream.as_mut())? {
            StoreKind::Plain => {
                IndexStore::PLAIN(PlainStore::new(header_stream.as_mut(), reader, pos_info)?)
            }
            _ => todo!(),
        })
    }

    pub fn get_entry(&self, idx: Idx<u32>) -> Result<Entry> {
        match self {
            IndexStore::PLAIN(store) => store.get_entry(idx),
            /*            _ => todo!()*/
        }
    }
}

pub struct PlainStore {
    pub variants: Vec<Vec<KeyDef>>,
    pub entry_reader: Box<dyn Reader>,
}

impl PlainStore {
    pub fn new(
        stream: &mut dyn Stream,
        reader: &dyn Reader,
        pos_info: SizedOffset,
    ) -> Result<Self> {
        let entry_size = stream.read_u16()?;
        let variant_count = Count::<u8>::produce(stream)?;
        let key_count = Count::<u8>::produce(stream)?;
        let mut variants = Vec::new();
        let mut entry_def = Vec::new();
        let mut current_size = 0;
        for _ in 0..key_count.0 {
            let key = KeyDef::produce(stream)?;
            current_size += key.size;
            entry_def.push(key);
            if current_size > entry_size {
                return Err(Error::FormatError);
            } else if current_size == entry_size {
                variants.push(entry_def);
                entry_def = Vec::new();
                current_size = 0;
            }
        }
        if !entry_def.is_empty() {
            variants.push(entry_def);
        }
        if variants.len() != variant_count.0 as usize {
            return Err(Error::FormatError);
        }
        let data_size = Size::produce(stream)?;
        // [TODO] use a array_reader here
        let entry_reader = reader.create_sub_reader(
            Offset(pos_info.offset.0 - data_size.0),
            End::Size(data_size),
        );
        Ok(Self {
            variants,
            entry_reader,
        })
    }

    pub fn get_entry(&self, id: Idx<u32>) -> Result<Entry> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1variant_allkeys() {
        #[rustfmt::skip]
        let content = vec![
            0x00, // kind
            0x05, 0x67,        //entry_size (1383)
            0x01,        // variant count
            0x15,        // key count
            0b1000_0000, // Variant id
            0b0000_0111, // padding key(8)
            0b0001_0000, // classic content address
            0b0001_0001, // patch content address
            0b0001_0010, // hidden content address
            0b0001_0011, // hidden+patch content address
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
            0b0111_0000, 0x0F, // PstringLookup(2), idx 0x0F
            0b0111_0111, 0x0F, // PstringLookup(9), idx 0x0F
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //data size
        ];
        let size = Size(content.len() as u64);
        let reader = Box::new(BufReader::new(content, End::None));
        let store = IndexStore::new(
            reader.as_ref(),
            SizedOffset {
                offset: Offset(0),
                size,
            },
        )
        .unwrap();
        let store = match store {
            IndexStore::PLAIN(s) => s,
        };
        assert_eq!(store.variants.len(), 1);
        let variant = &store.variants[0];
        let expected = vec![
            KeyDef {
                size: 1,
                kind: KeyKind::VariantId,
            },
            KeyDef {
                size: 8,
                kind: KeyKind::Padding,
            },
            KeyDef {
                size: 4,
                kind: KeyKind::ContentAddress {
                    patch: false,
                    hidden: false,
                },
            },
            KeyDef {
                size: 4,
                kind: KeyKind::ContentAddress {
                    patch: true,
                    hidden: false,
                },
            },
            KeyDef {
                size: 4,
                kind: KeyKind::ContentAddress {
                    patch: false,
                    hidden: true,
                },
            },
            KeyDef {
                size: 4,
                kind: KeyKind::ContentAddress {
                    patch: true,
                    hidden: true,
                },
            },
            KeyDef {
                size: 1,
                kind: KeyKind::UnsignedInt,
            },
            KeyDef {
                size: 3,
                kind: KeyKind::UnsignedInt,
            },
            KeyDef {
                size: 8,
                kind: KeyKind::UnsignedInt,
            },
            KeyDef {
                size: 1,
                kind: KeyKind::SignedInt,
            },
            KeyDef {
                size: 3,
                kind: KeyKind::SignedInt,
            },
            KeyDef {
                size: 8,
                kind: KeyKind::SignedInt,
            },
            KeyDef {
                size: 1,
                kind: KeyKind::CharArray,
            },
            KeyDef {
                size: 8,
                kind: KeyKind::CharArray,
            },
            KeyDef {
                size: 9,
                kind: KeyKind::CharArray,
            },
            KeyDef {
                size: 264,
                kind: KeyKind::CharArray,
            },
            KeyDef {
                size: 1032,
                kind: KeyKind::CharArray,
            },
            KeyDef {
                size: 1,
                kind: KeyKind::PString {
                    keystoreidx: 0x0F,
                    flookup: false,
                },
            },
            KeyDef {
                size: 8,
                kind: KeyKind::PString {
                    keystoreidx: 0x0F,
                    flookup: false,
                },
            },
            KeyDef {
                size: 2,
                kind: KeyKind::PString {
                    keystoreidx: 0x0F,
                    flookup: true,
                },
            },
            KeyDef {
                size: 9,
                kind: KeyKind::PString {
                    keystoreidx: 0x0F,
                    flookup: true,
                },
            },
        ];
        assert_eq!(variant, &expected);
    }

    #[test]
    fn test_2variants() {
        #[rustfmt::skip]
        let content = vec![
            0x00, // kind
            0x00, 0x1A,        //entry_size (26)
            0x02,        // variant count
            0x0C,        // key count (12)
            0b1000_0000, // Variant id
            0b0111_0011, 0x0F, // PstringLookup(5), idx 0x0F
            0b0100_1000, 0x00,        // char[9]
            0b0000_0011, // padding key(4)
            0b0001_0000, // classic content address
            0b0010_0010, // u24
            0b1000_0000, // Variant id
            0b0111_0011, 0x0F, // PstringLookup(5), idx 0x0F
            0b0100_1000, 0x00,        // char[9]
            0b0001_0010, // hidden content address
            0b0001_0001, // patch content address
            0b0010_0010, // u24
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, //data size
        ];
        let size = Size(content.len() as u64);
        let reader = Box::new(BufReader::new(content, End::None));
        let store = IndexStore::new(
            reader.as_ref(),
            SizedOffset {
                offset: Offset(0),
                size,
            },
        )
        .unwrap();
        let store = match store {
            IndexStore::PLAIN(s) => s,
        };
        assert_eq!(store.variants.len(), 2);
        let variant = &store.variants[0];
        let expected = vec![
            KeyDef {
                size: 1,
                kind: KeyKind::VariantId,
            },
            KeyDef {
                size: 5,
                kind: KeyKind::PString {
                    keystoreidx: 0x0F,
                    flookup: true,
                },
            },
            KeyDef {
                size: 9,
                kind: KeyKind::CharArray,
            },
            KeyDef {
                size: 4,
                kind: KeyKind::Padding,
            },
            KeyDef {
                size: 4,
                kind: KeyKind::ContentAddress {
                    patch: false,
                    hidden: false,
                },
            },
            KeyDef {
                size: 3,
                kind: KeyKind::UnsignedInt,
            },
        ];
        assert_eq!(variant, &expected);
        let variant = &store.variants[1];
        let expected = vec![
            KeyDef {
                size: 1,
                kind: KeyKind::VariantId,
            },
            KeyDef {
                size: 5,
                kind: KeyKind::PString {
                    keystoreidx: 0x0F,
                    flookup: true,
                },
            },
            KeyDef {
                size: 9,
                kind: KeyKind::CharArray,
            },
            KeyDef {
                size: 4,
                kind: KeyKind::ContentAddress {
                    patch: false,
                    hidden: true,
                },
            },
            KeyDef {
                size: 4,
                kind: KeyKind::ContentAddress {
                    patch: true,
                    hidden: false,
                },
            },
            KeyDef {
                size: 3,
                kind: KeyKind::UnsignedInt,
            },
        ];
        assert_eq!(variant, &expected);
    }
}
