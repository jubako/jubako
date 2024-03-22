use super::super::{PropertyName, VariantName};
use super::property::Property;
use super::Value;
use crate::bases::Writable;
use crate::bases::*;
use crate::creator::directory_pack::EntryTrait;

#[derive(Debug)]
#[repr(transparent)]
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
                    array_len_size,
                    fixed_array_len,
                    deported_info,
                    name,
                } => {
                    match entry.value(name).as_ref() {
                        Value::Array0(a) => {
                            if let Some(array_len_size) = array_len_size {
                                written += stream.write_usized(a.size as u64, *array_len_size)?;
                            }
                            written += stream.write_data(&a.data)?;
                            // Data is truncate at fixed_array_len. We just want to write 0 if data is shorter than fixed_array_len
                            written += stream.write_data(
                                vec![0; *fixed_array_len as usize - a.data.len()].as_slice(),
                            )?;
                            if let Some((key_size, _)) = deported_info {
                                written +=
                                    stream.write_usized(a.value_id.get().into_u64(), *key_size)?;
                            }
                        }
                        Value::Array1(a) => {
                            if let Some(array_len_size) = array_len_size {
                                written += stream.write_usized(a.size as u64, *array_len_size)?;
                            }
                            written += stream.write_data(&a.data)?;
                            // Data is truncate at fixed_array_len. We just want to write 0 if data is shorter than fixed_array_len
                            written += stream.write_data(
                                vec![0; *fixed_array_len as usize - a.data.len()].as_slice(),
                            )?;
                            if let Some((key_size, _)) = deported_info {
                                written +=
                                    stream.write_usized(a.value_id.get().into_u64(), *key_size)?;
                            }
                        }
                        Value::Array2(a) => {
                            if let Some(array_len_size) = array_len_size {
                                written += stream.write_usized(a.size as u64, *array_len_size)?;
                            }
                            written += stream.write_data(&a.data)?;
                            // Data is truncate at fixed_array_len. We just want to write 0 if data is shorter than fixed_array_len
                            written += stream.write_data(
                                vec![0; *fixed_array_len as usize - a.data.len()].as_slice(),
                            )?;
                            if let Some((key_size, _)) = deported_info {
                                written +=
                                    stream.write_usized(a.value_id.get().into_u64(), *key_size)?;
                            }
                        }
                        Value::Array(a) => {
                            if let Some(array_len_size) = array_len_size {
                                written += stream.write_usized(a.size as u64, *array_len_size)?;
                            }
                            written += stream.write_data(&a.data)?;
                            // Data is truncate at fixed_array_len. We just want to write 0 if data is shorter than fixed_array_len
                            written += stream.write_data(
                                vec![0; *fixed_array_len as usize - a.data.len()].as_slice(),
                            )?;
                            if let Some((key_size, _)) = deported_info {
                                written +=
                                    stream.write_usized(a.value_id.get().into_u64(), *key_size)?;
                            }
                        }
                        Value::IndirectArray(value_id) => {
                            assert_eq!(*array_len_size, None); // We don't store the size of the array
                            assert_eq!(*fixed_array_len, 0); // No fixed array
                            if let Some((key_size, _)) = deported_info {
                                written +=
                                    stream.write_usized(value_id.get().into_u64(), *key_size)?;
                            } else {
                                return Err(
                                    "A indirect array need a array property with a deported info"
                                        .to_string()
                                        .into(),
                                );
                            }
                        }
                        _ => {
                            return Err("Not a Array".to_string().into());
                        }
                    }
                }
                Property::IndirectArray {
                    value_id_size,
                    store_handle: _,
                    name,
                } => match entry.value(name).as_ref() {
                    Value::IndirectArray(value_id) => {
                        written +=
                            stream.write_usized(value_id.get().into_u64(), *value_id_size)?;
                    }
                    _ => {
                        return Err("Not a indirect Array".to_string().into());
                    }
                },
                Property::ContentAddress {
                    content_id_size,
                    pack_id_size,
                    default,
                    name,
                } => match entry.value(name).as_ref() {
                    Value::Content(value) => {
                        if let Some(d) = default {
                            assert_eq!(*d, value.pack_id.into_u16());
                        } else {
                            written +=
                                stream.write_usized(value.pack_id.into_u64(), *pack_id_size)?;
                        }
                        written +=
                            stream.write_usized(value.content_id.into_u64(), *content_id_size)?;
                    }
                    _ => {
                        return Err("Not a Content".to_string().into());
                    }
                },
                Property::UnsignedInt {
                    size,
                    default,
                    name,
                } => match entry.value(name).as_ref() {
                    Value::Unsigned(value) => {
                        if let Some(d) = default {
                            assert_eq!(d, value);
                        } else {
                            written += stream.write_usized(*value, *size)?;
                        }
                    }
                    Value::UnsignedWord(value) => {
                        if let Some(d) = default {
                            assert_eq!(*d, value.get());
                        } else {
                            written += stream.write_usized(value.get(), *size)?;
                        }
                    }
                    _ => {
                        return Err("Not a unsigned".to_string().into());
                    }
                },
                Property::SignedInt {
                    size,
                    default,
                    name,
                } => match entry.value(name).as_ref() {
                    Value::Signed(value) => {
                        if let Some(d) = default {
                            assert_eq!(d, value);
                        } else {
                            written += stream.write_isized(*value, *size)?;
                        }
                    }
                    Value::SignedWord(value) => {
                        if let Some(d) = default {
                            assert_eq!(*d, value.get());
                        } else {
                            written += stream.write_isized(value.get(), *size)?;
                        }
                    }
                    _ => {
                        return Err("Not a unsigned".to_string().into());
                    }
                },
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
