use crate::bases::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum ValueStoreKind {
    Plain = 0,
    Indexed = 1,
}

impl Producable for ValueStoreKind {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        match stream.read_u8()? {
            0 => Ok(ValueStoreKind::Plain),
            1 => Ok(ValueStoreKind::Indexed),
            v => Err(format_error!(
                &format!("Invalid ValueStoreKind ({})", v),
                stream
            )),
        }
    }
}

pub trait ValueStoreTrait {
    fn get_data(&self, id: ValueIdx) -> Result<Vec<u8>>;
}

pub enum ValueStore {
    Plain(PlainValueStore),
    Indexed(IndexedValueStore),
}

impl ValueStore {
    pub fn new(reader: &dyn Reader, pos_info: SizedOffset) -> Result<Self> {
        let header_reader =
            reader.create_sub_memory_reader(pos_info.offset, End::Size(pos_info.size))?;
        let mut header_stream = header_reader.create_stream_all();
        Ok(match ValueStoreKind::produce(header_stream.as_mut())? {
            ValueStoreKind::Plain => ValueStore::Plain(PlainValueStore::new(
                header_stream.as_mut(),
                reader,
                pos_info,
            )?),
            ValueStoreKind::Indexed => ValueStore::Indexed(IndexedValueStore::new(
                header_stream.as_mut(),
                reader,
                pos_info,
            )?),
        })
    }
}

impl ValueStoreTrait for ValueStore {
    fn get_data(&self, id: ValueIdx) -> Result<Vec<u8>> {
        match self {
            ValueStore::Plain(store) => store.get_data(id),
            ValueStore::Indexed(store) => store.get_data(id),
        }
    }
}

pub struct PlainValueStore {
    pub reader: Box<dyn Reader>,
}

impl PlainValueStore {
    fn new(stream: &mut dyn Stream, reader: &dyn Reader, pos_info: SizedOffset) -> Result<Self> {
        let data_size = Size::produce(stream)?;
        let reader =
            reader.create_sub_memory_reader(pos_info.offset - data_size, End::Size(data_size))?;
        Ok(PlainValueStore { reader })
    }

    fn get_data(&self, id: ValueIdx) -> Result<Vec<u8>> {
        let mut stream = self.reader.create_stream_from(Offset(id.into_u64()));
        PString::produce(stream.as_mut())
    }
}

pub struct IndexedValueStore {
    pub value_offsets: Vec<Offset>,
    pub reader: Box<dyn Reader>,
}

impl IndexedValueStore {
    fn new(stream: &mut dyn Stream, reader: &dyn Reader, pos_info: SizedOffset) -> Result<Self> {
        let value_count: ValueCount = Count::<u64>::produce(stream)?.into();
        let offset_size = stream.read_u8()?;
        let data_size: Size = stream.read_sized(offset_size.into())?.into();
        let value_count = value_count.into_usize();
        let mut value_offsets: Vec<Offset> = Vec::with_capacity(value_count + 1);
        // [TODO] Handle 32 and 16 bits
        let uninit = value_offsets.spare_capacity_mut();
        let mut first = true;
        for elem in &mut uninit[0..value_count] {
            let value: Offset = if first {
                first = false;
                0.into()
            } else {
                stream.read_sized(offset_size.into())?.into()
            };
            assert!(value.is_valid(data_size));
            elem.write(value);
        }
        unsafe { value_offsets.set_len(value_count) }
        value_offsets.push(data_size.into());
        assert_eq!(stream.tell().0, pos_info.size.into_u64());
        let reader =
            reader.create_sub_memory_reader(pos_info.offset - data_size, End::Size(data_size))?;
        Ok(IndexedValueStore {
            value_offsets,
            reader,
        })
    }

    fn get_data(&self, id: ValueIdx) -> Result<Vec<u8>> {
        let start = self.value_offsets[id.into_usize()];
        let end = self.value_offsets[id.into_usize() + 1];
        let mut stream = self.reader.create_stream(start, End::Offset(end));
        stream.read_vec((end - start).into_usize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valuestorekind() {
        let reader = BufReader::new(vec![0x00, 0x01, 0x02], End::None);
        let mut stream = reader.create_stream_all();
        assert_eq!(
            ValueStoreKind::produce(stream.as_mut()).unwrap(),
            ValueStoreKind::Plain
        );
        assert_eq!(
            ValueStoreKind::produce(stream.as_mut()).unwrap(),
            ValueStoreKind::Indexed
        );
        assert_eq!(stream.tell(), Offset::from(2));
        assert!(ValueStoreKind::produce(stream.as_mut()).is_err());
    }

    #[test]
    fn test_plainvaluestore() {
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
        let value_store =
            ValueStore::new(&reader, SizedOffset::new(Size::new(9), Offset(18))).unwrap();
        match &value_store {
            ValueStore::Plain(plainvaluestore) => {
                assert_eq!(plainvaluestore.reader.size(), Size::from(0x12_u64));
                assert_eq!(
                    plainvaluestore.reader.read_u64(Offset(0)).unwrap(),
                    0x0511121314150321_u64
                );
                assert_eq!(
                    plainvaluestore.reader.read_u64(Offset(8)).unwrap(),
                    0x2223073132333435_u64
                );
            }
            _ => panic!("Wrong type"),
        }

        assert_eq!(
            value_store.get_data(0.into()).unwrap(),
            vec![0x11, 0x12, 0x13, 0x14, 0x15]
        );
        assert_eq!(
            value_store.get_data(6.into()).unwrap(),
            vec![0x21, 0x22, 0x23]
        );
        assert_eq!(
            value_store.get_data(10.into()).unwrap(),
            vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
        );
    }

    #[test]
    fn test_indexedvaluestore() {
        #[rustfmt::skip]
        let reader = BufReader::new(
            vec![
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of entry 0
                0x21, 0x22, 0x23, // Data of entry 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of entry 2
                0x01, // kind
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, // value count
                0x01, // offset_size
                0x0f, // data_size
                0x05, // Offset of entry 1
                0x08, // Offset of entry 2
            ],
            End::None,
        );
        let value_store =
            ValueStore::new(&reader, SizedOffset::new(Size::new(13), Offset(15))).unwrap();
        match &value_store {
            ValueStore::Indexed(indexedvaluestore) => {
                assert_eq!(
                    indexedvaluestore.value_offsets,
                    vec![0.into(), 5.into(), 8.into(), 15.into()]
                );
                assert_eq!(indexedvaluestore.reader.size(), Size::from(0x0f_u64));
                assert_eq!(
                    indexedvaluestore.reader.read_u64(Offset(0)).unwrap(),
                    0x1112131415212223_u64
                );
                assert_eq!(
                    indexedvaluestore.reader.read_usized(Offset(8), 7).unwrap(),
                    0x31323334353637_u64
                );
            }
            _ => panic!("Wrong type"),
        }

        assert_eq!(
            value_store.get_data(0.into()).unwrap(),
            vec![0x11, 0x12, 0x13, 0x14, 0x15]
        );
        assert_eq!(
            value_store.get_data(1.into()).unwrap(),
            vec![0x21, 0x22, 0x23]
        );
        assert_eq!(
            value_store.get_data(2.into()).unwrap(),
            vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
        );
    }
}
