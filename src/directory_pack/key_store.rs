use crate::bases::producing::*;
use crate::bases::types::*;
use std::io::SeekFrom;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyStoreKind {
    PLAIN = 0,
    INDEXED = 1,
}

impl Producable for KeyStoreKind {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        match producer.read_u8()? {
            0 => Ok(KeyStoreKind::PLAIN),
            1 => Ok(KeyStoreKind::INDEXED),
            _ => {
                producer.seek(SeekFrom::Current(-1)).unwrap();
                Err(Error::FormatError)
            }
        }
    }
}

pub enum KeyStore {
    PLAIN(PlainKeyStore),
    INDEXED(IndexedKeyStore),
}

impl Producable for KeyStore {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        Ok(match KeyStoreKind::produce(producer)? {
            KeyStoreKind::PLAIN => KeyStore::PLAIN(PlainKeyStore::produce(producer)?),
            KeyStoreKind::INDEXED => KeyStore::INDEXED(IndexedKeyStore::produce(producer)?),
        })
    }
}

pub struct PlainKeyStore {
    pub producer: Box<dyn Producer>,
}

impl Producable for PlainKeyStore {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let data_size = Size::produce(producer)?;
        let producer = producer.sub_producer_at(producer.tell_cursor(), End::Size(data_size));
        Ok(PlainKeyStore { producer })
    }
}

pub struct IndexedKeyStore {
    pub entry_offsets: Vec<Offset>,
    pub producer: Box<dyn Producer>,
}

impl Producable for IndexedKeyStore {
    fn produce(producer: &mut dyn Producer) -> Result<Self> {
        let store_size = producer.read_u64()?;
        let entry_count: Count<u64> = Count::produce(producer)?;
        let offset_size = producer.read_u8()?;
        let data_size: Size = producer.read_sized(offset_size.into())?.into();
        let mut entry_offsets: Vec<Offset> = Vec::with_capacity((entry_count.0 + 1) as usize);
        // [TOOD] Handle 32 and 16 bits
        unsafe { entry_offsets.set_len(entry_count.0 as usize) }
        let mut first = true;
        for elem in entry_offsets.iter_mut() {
            if first {
                *elem = 0.into();
                first = false;
            } else {
                *elem = producer.read_sized(offset_size.into())?.into();
            }
            assert!(elem.is_valid(data_size));
        }
        entry_offsets.push(data_size.into());
        assert_eq!((producer.tell_cursor() + data_size).0, store_size);
        let producer =
            producer.sub_producer_at(producer.tell_cursor(), End::Offset(store_size.into()));
        Ok(IndexedKeyStore {
            entry_offsets,
            producer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::ProducerWrapper;

    #[test]
    fn test_keystorekind() {
        let mut producer = ProducerWrapper::<Vec<u8>>::new(vec![0x00, 0x01, 0x02], End::None);
        assert_eq!(
            KeyStoreKind::produce(&mut producer).unwrap(),
            KeyStoreKind::PLAIN
        );
        assert_eq!(
            KeyStoreKind::produce(&mut producer).unwrap(),
            KeyStoreKind::INDEXED
        );
        assert_eq!(producer.tell_cursor(), Offset::from(2));
        assert!(KeyStoreKind::produce(&mut producer).is_err());
        assert_eq!(producer.tell_cursor(), Offset::from(2));
        assert!(KeyStoreKind::produce(&mut producer).is_err());
    }

    #[test]
    fn test_plainkeystore() {
        let mut producer = ProducerWrapper::<Vec<u8>>::new(
            vec![
                0x00, // kind
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, // data_size
                0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, // data
                0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
            ],
            End::None,
        );
        let mut key_store = KeyStore::produce(&mut producer).unwrap();
        match &mut key_store {
            KeyStore::PLAIN(plainkeystore) => {
                assert_eq!(plainkeystore.producer.size(), Size::from(0x10_u64));
                assert_eq!(
                    plainkeystore.producer.read_u64().unwrap(),
                    0x1011121314151617_u64
                );
                assert_eq!(
                    plainkeystore.producer.read_u64().unwrap(),
                    0x18191a1b1c1d1e1f_u64
                );
            }
            _ => panic!("Wrong type"),
        }
    }

    #[test]
    fn test_indexedkeystore() {
        let mut producer = ProducerWrapper::<Vec<u8>>::new(
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
        println!("Size is {}", producer.size().0);
        let mut key_store = KeyStore::produce(&mut producer).unwrap();
        match &mut key_store {
            KeyStore::INDEXED(indexedkeystore) => {
                assert_eq!(
                    indexedkeystore.entry_offsets,
                    vec![0.into(), 5.into(), 8.into(), 15.into()]
                );
                assert_eq!(indexedkeystore.producer.size(), Size::from(0x0f_u64));
                assert_eq!(
                    indexedkeystore.producer.read_u64().unwrap(),
                    0x1112131415212223_u64
                );
                assert_eq!(
                    indexedkeystore.producer.read_sized(7).unwrap(),
                    0x31323334353637_u64
                );
            }
            _ => panic!("Wrong type"),
        }
    }
}
