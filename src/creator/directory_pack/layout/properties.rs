use super::property::Property;
use super::Value;
use crate::bases::Writable;
use crate::bases::*;
use crate::creator::directory_pack::{EntryIter, EntryTrait};

#[derive(Debug)]
pub struct Properties(Vec<Property>);

impl std::ops::Deref for Properties {
    type Target = [Property];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Properties {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<Property> for Properties {
    fn from_iter<I: IntoIterator<Item = Property>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl Properties {
    pub fn new(keys: Vec<Property>) -> Self {
        Self(keys)
    }

    pub fn write_entry<'a>(
        keys: impl Iterator<Item = &'a Property>,
        entry: &dyn EntryTrait,
        stream: &mut dyn OutStream,
    ) -> Result<usize> {
        let mut written = 0;
        let mut value_iter = EntryIter::new(entry);
        for key in keys {
            match key {
                Property::VLArray(flookup_size, store_handle) => {
                    let value = value_iter.next().unwrap();
                    if let Value::Array { data, value_id } = value {
                        written += stream.write_sized(
                            value_id.get(),
                            store_handle.borrow().key_size() as usize,
                        )?;
                        written += stream.write_data(data)?;
                        // Data is truncate at flookup_size. We just want to write 0 if data is shorter than flookup_size
                        written +=
                            stream.write_data(vec![0; *flookup_size - data.len()].as_slice())?;
                    } else {
                        return Err("Not a Array".to_string().into());
                    }
                }
                Property::ContentAddress => {
                    let value = value_iter.next().unwrap();
                    if let Value::Content(value) = value {
                        written += value.write(stream)?;
                    } else {
                        return Err("Not a Content".to_string().into());
                    }
                }
                Property::UnsignedInt(size) => {
                    let value = value_iter.next().unwrap();
                    if let Value::Unsigned(value) = value {
                        written += stream.write_sized(*value, *size as usize)?;
                    } else {
                        return Err("Not a unsigned".to_string().into());
                    }
                }
                Property::Padding(size) => {
                    let data = vec![0x00; *size as usize];
                    written += stream.write(&data)?;
                }
                Property::VariantId => {
                    written += stream.write_u8(entry.variant_id().unwrap().into_u8())?;
                }
            }
        }
        Ok(written)
    }

    pub(crate) fn entry_size(&self) -> u16 {
        self.iter().map(|k| k.size()).sum::<u16>()
    }

    pub(crate) fn key_count(&self) -> u8 {
        self.iter().map(|k| k.key_count()).sum::<u8>()
    }

    pub(crate) fn fill_to_size(&mut self, size: u16) {
        let current_size = self.entry_size();
        let mut padding_size = size - current_size;
        while padding_size >= 16 {
            self.0.push(Property::Padding(16));
            padding_size -= 16;
        }
        if padding_size > 0 {
            self.0.push(Property::Padding(padding_size as u8))
        }
    }
}

impl Writable for Properties {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        for key in &self.0 {
            written += key.write(stream)?;
        }
        Ok(written)
    }
}
