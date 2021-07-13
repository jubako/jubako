use crate::bases::reader::*;
use crate::bases::stream::*;
use crate::bases::types::*;
use std::io::SeekFrom;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyStoreKind {
    PLAIN = 0,
    INDEXED = 1,
}

impl Producable for KeyStoreKind {
    fn produce(stream: &mut dyn stream) -> Result<Self> {
        match stream.read_u8()? {
            0 => Ok(KeyStoreKind::PLAIN),
            1 => Ok(KeyStoreKind::INDEXED),
            _ => Err(Error::FormatError),
        }
    }
}

pub enum KeyStore {
    PLAIN(PlainKeyStore),
    INDEXED(IndexedKeyStore),
}

impl Producable for KeyStore {
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        Ok(match KeyStoreKind::produce(stream)? {
            KeyStoreKind::PLAIN => KeyStore::PLAIN(PlainKeyStore::produce(stream)?),
            KeyStoreKind::INDEXED => KeyStore::INDEXED(IndexedKeyStore::produce(stream)?),
        })
    }
}

pub struct PlainKeyStore {
    pub reader: Box<dyn Reader>,
}

impl PlainKeyStore {
    fn new(reader: Boy<dyn Reader>) -> Result<Self> {
        let mut stream = reader.create_stream(Offset(0), End::None);
        let data_size = Size::produce(stream.as_mut())?;
        let reader = reader.create_sub_reader(stream.tell(), End::Size(data_size));
        Ok(PlainKeyStore { reader })
    }
}

pub struct IndexedKeyStore {
    pub entry_offsets: Vec<Offset>,
    pub reader: Box<dyn Reader>,
}

impl IndexedKeyStore {
    fn new(reader: Box<dyn Reader>) -> Result<Self> {
        let mut stream = reader.create_stream(Offset(0), End::None);
        let store_size = stream.read_u64()?;
        let entry_count: Count<u64> = Count::produce(stream)?;
        let offset_size = stream.read_u8()?;
        let data_size: Size = stream.read_sized(offset_size.into())?.into();
        let mut entry_offsets: Vec<Offset> = Vec::with_capacity((entry_count.0 + 1) as usize);
        // [TOOD] Handle 32 and 16 bits
        unsafe { entry_offsets.set_len(entry_count.0 as usize) }
        let mut first = true;
        for elem in entry_offsets.iter_mut() {
            if first {
                *elem = 0.into();
                first = false;
            } else {
                *elem = stream.read_sized(offset_size.into())?.into();
            }
            assert!(elem.is_valid(data_size));
        }
        entry_offsets.push(data_size.into());
        assert_eq!((stream.tell() + data_size).0, store_size);
        let reader = reader.create_sub_reader(stream.tell(), End::Offset(store_size.into()));
        Ok(IndexedKeyStore {
            entry_offsets,
            reader,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keystorekind() {
        let reader = BufReader::new(vec![0x00, 0x01, 0x02], End::None);
        let mut stream = reader.create_stream(Offset(0), End::None);
        assert_eq!(
            KeyStoreKind::produce(stream.as_mut()).unwrap(),
            KeyStoreKind::PLAIN
        );
        assert_eq!(
            KeyStoreKind::produce(stream.as_mut()).unwrap(),
            KeyStoreKind::INDEXED
        );
        assert_eq!(stream.tell(), Offset::from(2));
        assert!(KeyStoreKind::produce(stream.as_mut()).is_err());
    }

    #[test]
    fn test_plainkeystore() {
        let reader = BufReader::new(
            vec![
                0x00, // kind
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, // data_size
                0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, // data
                0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
            ],
            End::None,
        );
        let key_store = KeyStore::new(reader).unwrap();
        match &key_store {
            KeyStore::PLAIN(plainkeystore) => {
                assert_eq!(plainkeystore.reader.size(), Size::from(0x10_u64));
                assert_eq!(
                    plainkeystore.read.read_u64(Offset(0)).unwrap(),
                    0x1011121314151617_u64
                );
                assert_eq!(
                    plainkeystore.read.read_u64(Offset(8)).unwrap(),
                    0x18191a1b1c1d1e1f_u64
                );
            }
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_indexedkeystore() {
        let reader = BufReader::new(
            vec![
                0x01, // kind
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x24, // store_size
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // key count
                0x01, // offset_size
                0x0f, // data_size
                0x05, // Offset of entry 1
                0x08, // Offset of entry 2
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of entry 0
                0x21, 0x22, 0x23, // Data of entry 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of entry 2
            ],
            End::None,
        );
        let key_store = KeyStore::new(reader).unwrap();
        match &key_store {
            KeyStore::INDEXED(indexedkeystore) => {
                assert_eq!(
                    indexedkeystore.entry_offsets,
                    vec![0.into(), 5.into(), 8.into(), 15.into()]
                );
                assert_eq!(indexedkeystore.reader.size(), Size::from(0x0f_u64));
                assert_eq!(
                    indexedkeystore.reader.read_u64(Offset(0)).unwrap(),
                    0x1112131415212223_u64
                );
                assert_eq!(
                    indexedkeystore.reader.read_sized(Offset(0), 7).unwrap(),
                    0x31323334353637_u64
                );
            }
            _ => panic!("Wrong type"),
        }
    }
}
