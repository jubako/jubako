use super::super::{PropertyName, VariantName};
use super::property::Property;
use super::Value;
use crate::bases::Writable;
use crate::bases::*;
use crate::creator::directory_pack::EntryTrait;

#[derive(Debug)]
pub struct Properties<PN: PropertyName>(Vec<Property<PN>>);

impl<PN: PropertyName> std::ops::Deref for Properties<PN> {
    type Target = [Property<PN>];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<PN: PropertyName> std::ops::DerefMut for Properties<PN> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<PN: PropertyName> FromIterator<Property<PN>> for Properties<PN> {
    fn from_iter<I: IntoIterator<Item = Property<PN>>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<PN: PropertyName> Properties<PN> {
    pub fn new(keys: Vec<Property<PN>>) -> Self {
        Self(keys)
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

impl<PN: PropertyName + 'static> Properties<PN> {
    pub fn write_entry<'a, VN: VariantName>(
        keys: impl Iterator<Item = &'a Property<PN>>,
        variant_id: Option<VariantIdx>,
        entry: &dyn EntryTrait<PN, VN>,
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
                    content_id_size,
                    pack_id_size,
                    default,
                    name,
                } => {
                    let value = entry.value(name);
                    if let Value::Content(value) = value {
                        if let Some(d) = default {
                            assert_eq!(*d, value.pack_id.into_u16());
                        } else {
                            written +=
                                stream.write_usized(value.pack_id.into_u64(), *pack_id_size)?;
                        }
                        written +=
                            stream.write_usized(value.content_id.into_u64(), *content_id_size)?;
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
                Property::VariantId(_name) => {
                    written += variant_id.unwrap().write(stream)?;
                }
            }
        }
        Ok(written)
    }
}

impl<PN: PropertyName> Writable for Properties<PN> {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        for key in &self.0 {
            written += key.write(stream)?;
        }
        Ok(written)
    }
}
