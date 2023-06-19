use super::property::Property;
use super::Value;
use crate::bases::Writable;
use crate::bases::*;
use crate::creator::directory_pack::EntryTrait;

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
        for key in keys {
            match key {
                Property::Array {
                    array_size_size,
                    fixed_array_size,
                    deported_info,
                    name,
                } => {
                    let value = entry.value(name);
                    if let Value::Array {
                        size,
                        data,
                        value_id,
                    } = value
                    {
                        if let Some(array_size_size) = array_size_size {
                            written += stream.write_usized(*size as u64, *array_size_size)?;
                        }
                        written += stream.write_data(data)?;
                        // Data is truncate at fixed_array_size. We just want to write 0 if data is shorter than fixed_array_size
                        written += stream.write_data(
                            vec![0; *fixed_array_size as usize - data.len()].as_slice(),
                        )?;
                        if let Some((key_size, _)) = deported_info {
                            written += stream.write_usized(value_id.get(), *key_size)?;
                        }
                    } else {
                        return Err("Not a Array".to_string().into());
                    }
                }
                Property::ContentAddress {
                    size,
                    default,
                    name,
                } => {
                    let value = entry.value(name);
                    if let Value::Content(value) = value {
                        if let Some(d) = default {
                            assert_eq!(*d, value.pack_id.into_u8());
                        } else {
                            written += stream.write_u8(value.pack_id.into_u8())?;
                        }
                        written += stream.write_usized(value.content_id.into_u64(), *size)?;
                    } else {
                        return Err("Not a Content".to_string().into());
                    }
                }
                Property::UnsignedInt {
                    size,
                    default,
                    name,
                } => {
                    let value = entry.value(name);
                    if let Value::Unsigned(value) = value {
                        if let Some(d) = default {
                            assert_eq!(*d, value.get());
                        } else {
                            written += stream.write_usized(value.get(), *size)?;
                        }
                    } else {
                        return Err("Not a unsigned".to_string().into());
                    }
                }
                Property::SignedInt {
                    size,
                    default,
                    name,
                } => {
                    let value = entry.value(name);
                    if let Value::Signed(value) = value {
                        if let Some(d) = default {
                            assert_eq!(*d, value.get());
                        } else {
                            written += stream.write_isized(value.get(), *size)?;
                        }
                    } else {
                        return Err("Not a unsigned".to_string().into());
                    }
                }
                Property::Padding(size) => {
                    let data = vec![0x00; *size as usize];
                    written += stream.write(&data)?;
                }
                Property::VariantId => {
                    written += entry.variant_id().unwrap().write(stream)?;
                }
            }
        }
        Ok(written)
    }

    pub(crate) fn entry_size(&self) -> u16 {
        self.iter().map(|k| k.size()).sum::<u16>()
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
