use crate::bases::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum KeyStoreKind {
    Plain = 0,
    Indexed = 1,
}

impl Producable for KeyStoreKind {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        match stream.read_u8()? {
            0 => Ok(KeyStoreKind::Plain),
            1 => Ok(KeyStoreKind::Indexed),
            v => Err(format_error!(
                &format!("Invalid KeyStoreKind ({})", v),
                stream
            )),
        }
    }
}

pub enum KeyStore {
    Plain(PlainKeyStore),
    Indexed(IndexedKeyStore),
}

impl KeyStore {
    pub fn new(reader: &dyn Reader, pos_info: SizedOffset) -> Result<Self> {
        let mut header_stream = reader.create_stream_for(pos_info);
        Ok(match KeyStoreKind::produce(header_stream.as_mut())? {
            KeyStoreKind::Plain => KeyStore::Plain(PlainKeyStore::new(
                header_stream.as_mut(),
                reader,
                pos_info,
            )?),
            KeyStoreKind::Indexed => KeyStore::Indexed(IndexedKeyStore::new(
                header_stream.as_mut(),
                reader,
                pos_info,
            )?),
        })
    }
    pub fn get_data(&self, id: Idx<u64>) -> Result<Vec<u8>> {
        match self {
            KeyStore::Plain(store) => store.get_data(id),
            KeyStore::Indexed(store) => store.get_data(id),
        }
    }
}

pub struct PlainKeyStore {
    pub reader: Box<dyn Reader>,
}

impl PlainKeyStore {
    fn new(stream: &mut dyn Stream, reader: &dyn Reader, pos_info: SizedOffset) -> Result<Self> {
        let data_size = Size::produce(stream)?;
        let reader = reader.create_sub_reader(
            Offset(pos_info.offset.0 - data_size.0),
            End::Size(data_size),
        );
        Ok(PlainKeyStore { reader })
    }

    fn get_data(&self, id: Idx<u64>) -> Result<Vec<u8>> {
        let mut stream = self.reader.create_stream_from(Offset(id.0));
        PString::produce(stream.as_mut())
    }
}

pub struct IndexedKeyStore {
    pub entry_offsets: Vec<Offset>,
    pub reader: Box<dyn Reader>,
}

impl IndexedKeyStore {
    fn new(stream: &mut dyn Stream, reader: &dyn Reader, pos_info: SizedOffset) -> Result<Self> {
        let entry_count = Count::<u64>::produce(stream)?;
        let offset_size = stream.read_u8()?;
        let data_size: Size = stream.read_sized(offset_size.into())?.into();
        let entry_count = entry_count.0 as usize;
        let mut entry_offsets: Vec<Offset> = Vec::with_capacity(entry_count + 1);
        // [TODO] Handle 32 and 16 bits
        let uninit = entry_offsets.spare_capacity_mut();
        let mut first = true;
        for elem in &mut uninit[0..entry_count] {
            let value: Offset = if first {
                first = false;
                0.into()
            } else {
                stream.read_sized(offset_size.into())?.into()
            };
            assert!(value.is_valid(data_size));
            elem.write(value);
        }
        unsafe { entry_offsets.set_len(entry_count) }
        entry_offsets.push(data_size.into());
        assert_eq!(stream.tell().0, pos_info.size.0);
        let reader = reader.create_sub_reader(
            Offset(pos_info.offset.0 - data_size.0),
            End::Size(data_size),
        );
        Ok(IndexedKeyStore {
            entry_offsets,
            reader,
        })
    }

    fn get_data(&self, id: Idx<u64>) -> Result<Vec<u8>> {
        let start = self.entry_offsets[id.0 as usize];
        let end = self.entry_offsets[(id.0 + 1) as usize];
        let mut stream = self.reader.create_stream(start, End::Offset(end));
        stream.read_vec((end - start).0 as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keystorekind() {
        let reader = BufReader::new(vec![0x00, 0x01, 0x02], End::None);
        let mut stream = reader.create_stream_all();
        assert_eq!(
            KeyStoreKind::produce(stream.as_mut()).unwrap(),
            KeyStoreKind::Plain
        );
        assert_eq!(
            KeyStoreKind::produce(stream.as_mut()).unwrap(),
            KeyStoreKind::Indexed
        );
        assert_eq!(stream.tell(), Offset::from(2));
        assert!(KeyStoreKind::produce(stream.as_mut()).is_err());
    }

    #[test]
    fn test_plainkeystore() {
        #[rustfmt::skip]
        let reader = BufReader::new(
            vec![
                0x05, 0x11, 0x12, 0x13, 0x14, 0x15, // Data of entry 0
                0x03, 0x21, 0x22, 0x23, // Data of entry 1
                0x07, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of entry 2
                0x00, // kind
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x12, // data_size
            ],
            End::None,
        );
        let key_store = KeyStore::new(&reader, SizedOffset::new(Size(9), Offset(18))).unwrap();
        match &key_store {
            KeyStore::Plain(plainkeystore) => {
                assert_eq!(plainkeystore.reader.size(), Size::from(0x12_u64));
                assert_eq!(
                    plainkeystore.reader.read_u64(Offset(0)).unwrap(),
                    0x0511121314150321_u64
                );
                assert_eq!(
                    plainkeystore.reader.read_u64(Offset(8)).unwrap(),
                    0x2223073132333435_u64
                );
            }
            _ => panic!("Wrong type"),
        }

        assert_eq!(
            key_store.get_data(0.into()).unwrap(),
            vec![0x11, 0x12, 0x13, 0x14, 0x15]
        );
        assert_eq!(
            key_store.get_data(6.into()).unwrap(),
            vec![0x21, 0x22, 0x23]
        );
        assert_eq!(
            key_store.get_data(10.into()).unwrap(),
            vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
        );
    }

    #[test]
    fn test_indexedkeystore() {
        #[rustfmt::skip]
        let reader = BufReader::new(
            vec![
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of entry 0
                0x21, 0x22, 0x23, // Data of entry 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of entry 2
                0x01, // kind
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // key count
                0x01, // offset_size
                0x0f, // data_size
                0x05, // Offset of entry 1
                0x08, // Offset of entry 2
            ],
            End::None,
        );
        let key_store = KeyStore::new(&reader, SizedOffset::new(Size(13), Offset(15))).unwrap();
        match &key_store {
            KeyStore::Indexed(indexedkeystore) => {
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
                    indexedkeystore.reader.read_usized(Offset(8), 7).unwrap(),
                    0x31323334353637_u64
                );
            }
            _ => panic!("Wrong type"),
        }

        assert_eq!(
            key_store.get_data(0.into()).unwrap(),
            vec![0x11, 0x12, 0x13, 0x14, 0x15]
        );
        assert_eq!(
            key_store.get_data(1.into()).unwrap(),
            vec![0x21, 0x22, 0x23]
        );
        assert_eq!(
            key_store.get_data(2.into()).unwrap(),
            vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
        );
    }
}
