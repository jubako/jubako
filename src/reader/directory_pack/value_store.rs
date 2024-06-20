use crate::bases::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum ValueStoreKind {
    Plain = 0,
    Indexed = 1,
}

impl Producable for ValueStoreKind {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        match flux.read_u8()? {
            0 => Ok(ValueStoreKind::Plain),
            1 => Ok(ValueStoreKind::Indexed),
            v => Err(format_error!(
                &format!("Invalid ValueStoreKind ({v})"),
                flux
            )),
        }
    }
}

pub trait ValueStoreTrait: std::fmt::Debug + Send + Sync {
    fn get_data(&self, id: ValueIdx, size: Option<Size>) -> Result<&[u8]>;
}

#[derive(Debug)]
#[cfg_attr(feature = "explorable", derive(serde::Serialize))]
pub enum ValueStore {
    Plain(PlainValueStore),
    Indexed(IndexedValueStore),
}

impl ValueStore {
    pub fn new(reader: &Reader, pos_info: SizedOffset) -> Result<Self> {
        let header_reader = reader.create_sub_memory_reader(pos_info.offset, pos_info.size)?;
        let mut header_flux = header_reader.create_flux_all();
        Ok(match ValueStoreKind::produce(&mut header_flux)? {
            ValueStoreKind::Plain => {
                ValueStore::Plain(PlainValueStore::new(&mut header_flux, reader, pos_info)?)
            }
            ValueStoreKind::Indexed => {
                ValueStore::Indexed(IndexedValueStore::new(&mut header_flux, reader, pos_info)?)
            }
        })
    }
}

impl ValueStoreTrait for ValueStore {
    fn get_data(&self, id: ValueIdx, size: Option<Size>) -> Result<&[u8]> {
        match self {
            ValueStore::Plain(store) => store.get_data(id, size),
            ValueStore::Indexed(store) => store.get_data(id, size),
        }
    }
}

#[cfg(feature = "explorable")]
impl Explorable for ValueStore {
    fn explore_one(&self, item: &str) -> Result<Option<Box<dyn Explorable>>> {
        match self {
            ValueStore::Plain(store) => store.explore_one(item),
            ValueStore::Indexed(store) => store.explore_one(item),
        }
    }
}

#[derive(Debug)]
pub struct PlainValueStore {
    pub reader: MemoryReader,
}

impl PlainValueStore {
    fn new(flux: &mut Flux, reader: &Reader, pos_info: SizedOffset) -> Result<Self> {
        let data_size = Size::produce(flux)?;
        let reader = reader.create_sub_memory_reader(pos_info.offset - data_size, data_size)?;
        Ok(PlainValueStore {
            reader: reader.try_into()?,
        })
    }

    fn get_data(&self, id: ValueIdx, size: Option<Size>) -> Result<&[u8]> {
        if let Some(size) = size {
            let offset = id.into_u64().into();
            self.reader.get_slice(offset, size)
        } else {
            panic!("Cannot use unsized with PlainValueStore");
        }
    }
}

#[cfg(feature = "explorable")]
impl serde::Serialize for PlainValueStore {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut ser = serializer.serialize_struct("PlainValueStore", 1)?;
        ser.serialize_field("size", &self.reader.size())?;
        ser.end()
    }
}

#[cfg(feature = "explorable")]
impl Explorable for PlainValueStore {
    fn explore_one(&self, item: &str) -> Result<Option<Box<dyn Explorable>>> {
        if let Some((first, second)) = item.split_once('-') {
            let offset = first
                .parse::<u64>()
                .map_err(|e| Error::from(format!("{e}")))?;
            let size = second
                .parse::<u64>()
                .map_err(|e| Error::from(format!("{e}")))?;
            if offset > self.reader.size().into_u64()
                || (offset + size > self.reader.size().into_u64())
            {
                return Ok(None);
            }
            Ok(Some(Box::new(
                String::from_utf8_lossy(
                    self.get_data(ValueIdx::from(offset), Some(Size::from(size)))?,
                )
                .into_owned(),
            )))
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
pub struct IndexedValueStore {
    pub value_offsets: Vec<Offset>,
    pub reader: MemoryReader,
}

impl IndexedValueStore {
    fn new(flux: &mut Flux, reader: &Reader, pos_info: SizedOffset) -> Result<Self> {
        let value_count: ValueCount = Count::<u64>::produce(flux)?.into();
        let offset_size = ByteSize::produce(flux)?;
        let data_size: Size = flux.read_usized(offset_size)?.into();
        let value_count = value_count.into_usize();
        // [FIXME] A lot of value means a lot of allocation.
        // A wrong value here (or a carefully choosen one) may break our program.
        let mut value_offsets: Vec<Offset> = Vec::with_capacity(value_count + 1);
        // [TODO] Handle 32 and 16 bits
        let uninit = value_offsets.spare_capacity_mut();
        let mut first = true;
        for elem in &mut uninit[0..value_count] {
            let value: Offset = if first {
                first = false;
                Offset::zero()
            } else {
                flux.read_usized(offset_size)?.into()
            };
            assert!(value.is_valid(data_size));
            elem.write(value);
        }
        unsafe { value_offsets.set_len(value_count) }
        value_offsets.push(data_size.into());
        assert_eq!(flux.tell().into_u64(), pos_info.size.into_u64());
        let reader = reader.create_sub_memory_reader(pos_info.offset - data_size, data_size)?;
        Ok(IndexedValueStore {
            value_offsets,
            reader: reader.try_into()?,
        })
    }

    fn get_data(&self, id: ValueIdx, size: Option<Size>) -> Result<&[u8]> {
        if id.into_usize() + 1 >= self.value_offsets.len() {
            return Err(format_error!(&format!("{id} is not a valid id")));
        }
        let start = self.value_offsets[id.into_usize()];
        let size = match size {
            Some(s) => s,
            None => self.value_offsets[id.into_usize() + 1] - start,
        };
        self.reader.get_slice(start, size)
    }
}

#[cfg(feature = "explorable")]
impl serde::Serialize for IndexedValueStore {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut ser = serializer.serialize_struct("IndexedValueStore", 1)?;
        ser.serialize_field("offsets", &(self.value_offsets.len() - 1))?;
        ser.end()
    }
}

#[cfg(feature = "explorable")]
impl Explorable for IndexedValueStore {
    fn explore_one(&self, item: &str) -> Result<Option<Box<dyn Explorable>>> {
        let (idx, size) = if let Some((first, second)) = item.split_once('-') {
            let offset = first
                .parse::<u64>()
                .map_err(|e| Error::from(format!("{e}")))?;
            let size = Some(Size::from(
                second
                    .parse::<u64>()
                    .map_err(|e| Error::from(format!("{e}")))?,
            ));
            (offset, size)
        } else {
            let offset = item
                .parse::<u64>()
                .map_err(|e| Error::from(format!("{e}")))?;
            (offset, None)
        };
        if idx >= self.value_offsets.len() as u64 {
            return Err(format!("{idx} is not a valid index").into());
        }
        Ok(Some(Box::new(
            String::from_utf8_lossy(self.get_data(ValueIdx::from(idx), size)?).into_owned(),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valuestorekind() {
        let reader = Reader::from(vec![0x00, 0x01, 0x02]);
        let mut flux = reader.create_flux_all();
        assert_eq!(
            ValueStoreKind::produce(&mut flux).unwrap(),
            ValueStoreKind::Plain
        );
        assert_eq!(
            ValueStoreKind::produce(&mut flux).unwrap(),
            ValueStoreKind::Indexed
        );
        assert_eq!(flux.tell(), Offset::new(2));
        assert!(ValueStoreKind::produce(&mut flux).is_err());
    }

    #[test]
    fn test_plainvaluestore() {
        #[rustfmt::skip]
        let reader = Reader::from(
            vec![
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of entry 0
                0x21, 0x22, 0x23, // Data of entry 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of entry 2
                0x00, // kind
                0x0F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // data_size
            ]
        );
        let value_store =
            ValueStore::new(&reader, SizedOffset::new(Size::new(9), Offset::new(15))).unwrap();
        match &value_store {
            ValueStore::Plain(plainvaluestore) => {
                assert_eq!(plainvaluestore.reader.size(), Size::from(0x0F_u64));
                assert_eq!(
                    plainvaluestore.reader.read_u64(Offset::zero()).unwrap(),
                    0x2322211514131211_u64
                );
                assert_eq!(
                    plainvaluestore.reader.read_u64(Offset::new(7)).unwrap(),
                    0x3736353433323123_u64
                );
            }
            _ => panic!("Wrong type"),
        }

        assert_eq!(
            value_store.get_data(0.into(), Some(Size::new(5))).unwrap(),
            vec![0x11, 0x12, 0x13, 0x14, 0x15]
        );
        assert_eq!(
            value_store.get_data(5.into(), Some(Size::new(3))).unwrap(),
            vec![0x21, 0x22, 0x23]
        );
        assert_eq!(
            value_store.get_data(8.into(), Some(Size::new(7))).unwrap(),
            vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
        );
    }

    #[test]
    fn test_indexedvaluestore() {
        #[rustfmt::skip]
        let reader = Reader::from(
            vec![
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of entry 0
                0x21, 0x22, 0x23, // Data of entry 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of entry 2
                0x01, // kind
                0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // value count
                0x01, // offset_size
                0x0f, // data_size
                0x05, // Offset of entry 1
                0x08, // Offset of entry 2
            ]
        );
        let value_store =
            ValueStore::new(&reader, SizedOffset::new(Size::new(13), Offset::new(15))).unwrap();
        match &value_store {
            ValueStore::Indexed(indexedvaluestore) => {
                assert_eq!(
                    indexedvaluestore.value_offsets,
                    vec![0_u64.into(), 5_u64.into(), 8_u64.into(), 15_u64.into()]
                );
                assert_eq!(indexedvaluestore.reader.size(), Size::from(0x0f_u64));
                assert_eq!(
                    indexedvaluestore.reader.read_u64(Offset::zero()).unwrap(),
                    0x2322211514131211_u64
                );
                assert_eq!(
                    indexedvaluestore
                        .reader
                        .read_usized(Offset::new(8), ByteSize::U7)
                        .unwrap(),
                    0x37363534333231_u64
                );
            }
            _ => panic!("Wrong type"),
        }

        assert_eq!(
            value_store.get_data(0.into(), None).unwrap(),
            vec![0x11, 0x12, 0x13, 0x14, 0x15]
        );
        assert_eq!(
            value_store.get_data(1.into(), None).unwrap(),
            vec![0x21, 0x22, 0x23]
        );
        assert_eq!(
            value_store.get_data(2.into(), None).unwrap(),
            vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
        );

        assert_eq!(
            value_store.get_data(0.into(), Some(Size::new(5))).unwrap(),
            vec![0x11, 0x12, 0x13, 0x14, 0x15]
        );
        assert_eq!(
            value_store.get_data(1.into(), Some(Size::new(3))).unwrap(),
            vec![0x21, 0x22, 0x23]
        );
        assert_eq!(
            value_store.get_data(2.into(), Some(Size::new(7))).unwrap(),
            vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
        );
    }
}
