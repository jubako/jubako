use std::borrow::Cow;

use crate::bases::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum ValueStoreKind {
    Plain = 0,
    Indexed = 1,
}

impl Parsable for ValueStoreKind {
    type Output = Self;
    fn parse(parser: &mut impl Parser) -> Result<Self> {
        match parser.read_u8()? {
            0 => Ok(ValueStoreKind::Plain),
            1 => Ok(ValueStoreKind::Indexed),
            v => Err(format_error!(
                &format!("Invalid ValueStoreKind ({v})"),
                parser
            )),
        }
    }
}

pub trait ValueStoreTrait: std::fmt::Debug + Send + Sync {
    fn get_data(&self, id: ValueIdx, size: Option<ASize>) -> Result<&[u8]>;
}

#[derive(Debug)]
#[cfg_attr(feature = "explorable_serde", derive(serde::Serialize))]
pub enum ValueStore {
    Plain(PlainValueStore),
    Indexed(IndexedValueStore),
}

impl ValueStoreTrait for ValueStore {
    fn get_data(&self, id: ValueIdx, size: Option<ASize>) -> Result<&[u8]> {
        match self {
            ValueStore::Plain(store) => store.get_data(id, size),
            ValueStore::Indexed(store) => store.get_data(id, size),
        }
    }
}

#[cfg(feature = "explorable")]
impl graphex::Node for ValueStore {
    fn next(&self, key: &str) -> graphex::ExploreResult {
        match self {
            ValueStore::Plain(store) => store.next(key),
            ValueStore::Indexed(store) => store.next(key),
        }
    }
    fn display(&self) -> &dyn graphex::Display {
        match self {
            ValueStore::Plain(store) => store.display(),
            ValueStore::Indexed(store) => store.display(),
        }
    }

    #[cfg(feature = "explorable_serde")]
    fn serde(&self) -> Option<&dyn erased_serde::Serialize> {
        match self {
            ValueStore::Plain(store) => store.serde(),
            ValueStore::Indexed(store) => store.serde(),
        }
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for ValueStore {
    fn header_footer(&self) -> Option<(String, String)> {
        match self {
            ValueStore::Plain(store) => store.header_footer(),
            ValueStore::Indexed(store) => store.header_footer(),
        }
    }
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        match self {
            ValueStore::Plain(store) => store.print_content(out),
            ValueStore::Indexed(store) => store.print_content(out),
        }
    }
}

pub(crate) enum ValueStoreBuilder {
    Plain,
    Indexed(Vec<Offset>),
}

impl Parsable for ValueStoreBuilder {
    type Output = (Self, Size);
    fn parse(parser: &mut impl Parser) -> Result<Self::Output>
    where
        Self::Output: Sized,
    {
        let kind = ValueStoreKind::parse(parser)?;
        match kind {
            ValueStoreKind::Plain => {
                let data_size = Size::parse(parser)?;
                Ok((ValueStoreBuilder::Plain, data_size))
            }
            ValueStoreKind::Indexed => {
                let value_count: ValueCount = Count::<u64>::parse(parser)?.into();
                let offset_size = ByteSize::parse(parser)?;
                let data_size: Size = parser.read_usized(offset_size)?.into();

                #[cfg(target_pointer_width = "64")]
                let value_count = value_count.into_u64() as usize;

                // < instead of <= because of the +1 just after
                #[cfg(target_pointer_width = "32")]
                let value_count = if value_count.into_u64() < usize::MAX as u64 {
                    value_count.into_u64() as usize
                } else {
                    unimplemented!()
                };

                // [FIXME] A lot of value means a lot of allocation.
                // A wrong value here (or a carefully choosen one) may break our program.
                let mut value_offsets: Vec<Offset> = Vec::with_capacity(value_count + 1);
                let uninit = value_offsets.spare_capacity_mut();
                let mut first = true;
                for elem in &mut uninit[0..value_count] {
                    let value: Offset = if first {
                        first = false;
                        Offset::zero()
                    } else {
                        parser.read_usized(offset_size)?.into()
                    };
                    assert!(value.is_valid(data_size));
                    elem.write(value);
                }
                unsafe { value_offsets.set_len(value_count) }
                value_offsets.push(data_size.into());
                Ok((ValueStoreBuilder::Indexed(value_offsets), data_size))
            }
        }
    }
}

impl BlockParsable for ValueStoreBuilder {}

impl DataBlockParsable for ValueStore {
    type TailParser = ValueStoreBuilder;
    type Output = Self;

    fn finalize(
        intermediate: (ValueStoreBuilder, Size),
        header_offset: Offset,
        reader: &Reader,
    ) -> Result<Self::Output> {
        let (store_builder, data_size) = intermediate;
        let reader = reader.cut_check(
            header_offset - data_size - ASize::from(BlockCheck::Crc32.size()),
            data_size,
            BlockCheck::Crc32,
        )?;
        // We want to be sure that we load all the ValueStore data in memory first.
        Ok(match store_builder {
            ValueStoreBuilder::Plain => Self::Plain(PlainValueStore { reader }),
            ValueStoreBuilder::Indexed(value_offsets) => Self::Indexed(IndexedValueStore {
                value_offsets,
                reader,
            }),
        })
    }
}

#[derive(Debug)]
pub struct PlainValueStore {
    reader: CheckReader,
}

impl PlainValueStore {
    pub(self) fn get_data(&self, id: ValueIdx, size: Option<ASize>) -> Result<&[u8]> {
        if let Some(size) = size {
            let offset = id.into_u64().into();
            if let Cow::Borrowed(s) = self.reader.get_slice(offset, size)? {
                Ok(s)
            } else {
                unreachable!("Reader must be from memory")
            }
        } else {
            panic!("Cannot use unsized with PlainValueStore");
        }
    }
}

#[cfg(feature = "explorable_serde")]
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
impl graphex::Display for PlainValueStore {
    fn header_footer(&self) -> Option<(String, String)> {
        Some(("PlainValueStore(".to_string(), ")".to_string()))
    }
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        use yansi::Paint;
        out.field(
            &format!("size ({})", "<offset>-<len>".bold()),
            &self.reader.size(),
        )
    }
}

#[cfg(feature = "explorable")]
impl graphex::Node for PlainValueStore {
    fn next(&self, key: &str) -> graphex::ExploreResult {
        if let Some((first, second)) = key.split_once('-') {
            let offset = first
                .parse::<u64>()
                .map_err(|e| graphex::Error::key(&format!("{e}")))?;
            let size = second
                .parse::<usize>()
                .map_err(|e| graphex::Error::key(&format!("{e}")))?;
            if offset > self.reader.size().into_u64()
                || (offset + size as u64 > self.reader.size().into_u64())
            {
                return Err(graphex::Error::key(key));
            }
            Ok(Box::new(
                String::from_utf8_lossy(
                    self.get_data(ValueIdx::from(offset), Some(ASize::from(size)))?,
                )
                .into_owned(),
            )
            .into())
        } else {
            Err(graphex::Error::key(key))
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

#[derive(Debug)]
pub struct IndexedValueStore {
    value_offsets: Vec<Offset>,
    reader: CheckReader,
}

impl IndexedValueStore {
    pub(self) fn get_data(&self, id: ValueIdx, size: Option<ASize>) -> Result<&[u8]> {
        #[cfg(target_pointer_width = "32")]
        if id.into_u64() > usize::MAX as u64 {
            unimplemented!();
        }
        let id = id.into_u64() as usize;
        if id + 1 >= self.value_offsets.len() {
            return Err(format_error!(&format!("{id} is not a valid id")));
        }
        let start = self.value_offsets[id];
        let size = match size {
            Some(s) => s,
            None => {
                let s = self.value_offsets[id + 1] - start;
                assert!(s.into_u64() <= usize::MAX as u64);
                ASize::new(s.into_u64() as usize)
            }
        };
        if let Cow::Borrowed(s) = self.reader.get_slice(start, size)? {
            Ok(s)
        } else {
            unreachable!("Reader must be from memory")
        }
    }
}

#[cfg(feature = "explorable_serde")]
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
impl graphex::Display for IndexedValueStore {
    fn header_footer(&self) -> Option<(String, String)> {
        Some(("IndexedValueStore(".to_string(), ")".to_string()))
    }
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        use yansi::Paint;
        out.field(
            &format!("values count ({})", "<N> or <N>-<len>".bold()),
            &(self.value_offsets.len() - 1),
        )
    }
}

#[cfg(feature = "explorable")]
impl graphex::Node for IndexedValueStore {
    fn next(&self, key: &str) -> graphex::ExploreResult {
        let (idx, size) = if let Some((first, second)) = key.split_once('-') {
            let offset = first
                .parse::<u64>()
                .map_err(|e| graphex::Error::key(&format!("{e}")))?;
            let size = Some(ASize::from(
                second
                    .parse::<usize>()
                    .map_err(|e| graphex::Error::key(&format!("{e}")))?,
            ));
            (offset, size)
        } else {
            let offset = key
                .parse::<u64>()
                .map_err(|e| graphex::Error::key(&format!("{e}")))?;
            (offset, None)
        };
        if idx >= self.value_offsets.len() as u64 {
            return Err(graphex::Error::key(&format!("{idx} is not a valid index")));
        }
        Ok(Box::new(
            String::from_utf8_lossy(self.get_data(ValueIdx::from(idx), size)?).into_owned(),
        )
        .into())
    }

    fn display(&self) -> &dyn graphex::Display {
        self
    }

    #[cfg(feature = "explorable_serde")]
    fn serde(&self) -> Option<&dyn erased_serde::Serialize> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustest::test]
    fn test_valuestorekind() {
        let reader = CheckReader::from([0x00, 0x01, 0x02]);
        let mut parser = reader.create_parser(Offset::zero(), 3.into()).unwrap();
        assert_eq!(
            ValueStoreKind::parse(&mut parser).unwrap(),
            ValueStoreKind::Plain
        );
        assert_eq!(
            ValueStoreKind::parse(&mut parser).unwrap(),
            ValueStoreKind::Indexed
        );
        assert_eq!(parser.tell(), Offset::new(2));
        assert!(ValueStoreKind::parse(&mut parser).is_err());
    }

    #[rustest::test]
    fn test_plainvaluestore() {
        #[rustfmt::skip]
        let reader = Reader::from(
            vec![
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of entry 0
                0x21, 0x22, 0x23, // Data of entry 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of entry 2
                0x0D, 0x0D, 0x73, 0xA0, // CRC
                0x00, // kind
                0x0F, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // data_size
                0xE4, 0x65, 0xB6, 0xC7, // CRC
            ]
        );
        let value_store = reader
            .parse_data_block::<ValueStore>(SizedOffset::new(ASize::new(9), Offset::new(19)))
            .unwrap();
        match &value_store {
            ValueStore::Plain(plainvaluestore) => {
                assert_eq!(plainvaluestore.reader.size(), Size::from(0x0F_u64));
                assert_eq!(
                    plainvaluestore
                        .reader
                        .get_slice(Offset::zero(), ASize::new(8))
                        .unwrap()
                        .as_ref(),
                    &[0x11, 0x12, 0x13, 0x14, 0x15, 0x21, 0x22, 0x23]
                );
                assert_eq!(
                    plainvaluestore
                        .reader
                        .get_slice(Offset::new(7), ASize::new(5))
                        .unwrap()
                        .as_ref(),
                    &[0x23, 0x31, 0x32, 0x33, 0x34]
                );
            }
            _ => panic!("Wrong type"),
        }

        assert_eq!(
            value_store.get_data(0.into(), Some(5.into())).unwrap(),
            vec![0x11, 0x12, 0x13, 0x14, 0x15]
        );
        assert_eq!(
            value_store.get_data(5.into(), Some(3.into())).unwrap(),
            vec![0x21, 0x22, 0x23]
        );
        assert_eq!(
            value_store.get_data(8.into(), Some(7.into())).unwrap(),
            vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
        );
    }

    #[rustest::test]
    fn test_indexedvaluestore() {
        #[rustfmt::skip]
        let reader = Reader::from(
            vec![
                0x11, 0x12, 0x13, 0x14, 0x15, // Data of entry 0
                0x21, 0x22, 0x23, // Data of entry 1
                0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, // Data of entry 2
                0x0D, 0x0D, 0x73, 0xA0, // CRC
                0x01, // kind
                0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // value count
                0x01, // offset_size
                0x0f, // data_size
                0x05, // Offset of entry 1
                0x08, // Offset of entry 2
                0x1E, 0x6E, 0xE7, 0xB7, // CRC
            ]
        );
        let value_store = reader
            .parse_data_block::<ValueStore>(SizedOffset::new(ASize::new(13), Offset::new(19)))
            .unwrap();
        match &value_store {
            ValueStore::Indexed(indexedvaluestore) => {
                assert_eq!(
                    indexedvaluestore.value_offsets,
                    vec![0_u64.into(), 5_u64.into(), 8_u64.into(), 15_u64.into()]
                );
                assert_eq!(indexedvaluestore.reader.size(), Size::from(0x0f_u64));
                assert_eq!(
                    indexedvaluestore
                        .reader
                        .get_slice(Offset::zero(), ASize::new(8))
                        .unwrap()
                        .as_ref(),
                    &[0x11, 0x12, 0x13, 0x14, 0x15, 0x21, 0x22, 0x23]
                );
                assert_eq!(
                    indexedvaluestore
                        .reader
                        .get_slice(Offset::new(8), ASize::from(ByteSize::U7 as usize))
                        .unwrap()
                        .as_ref(),
                    &[0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
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
            value_store.get_data(0.into(), Some(5.into())).unwrap(),
            vec![0x11, 0x12, 0x13, 0x14, 0x15]
        );
        assert_eq!(
            value_store.get_data(1.into(), Some(3.into())).unwrap(),
            vec![0x21, 0x22, 0x23]
        );
        assert_eq!(
            value_store.get_data(2.into(), Some(7.into())).unwrap(),
            vec![0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37]
        );
    }
}
