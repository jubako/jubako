use super::entry::Entry;
use super::key::{Key, KeyKind};
use super::key_def::{KeyDef, KeyDefKind};
use crate::bases::*;
use std::cmp::Ordering;

#[derive(Debug)]
pub struct VariantDef {
    pub keys: Vec<Key>,
}

impl VariantDef {
    pub fn new(keydefs: Vec<KeyDef>) -> Result<Self> {
        let mut offset = 0;
        let mut current_idx = 0;
        let mut keys = Vec::new();
        while current_idx < keydefs.len() {
            let (key, new_idx, new_offset) = Self::build_key(current_idx, offset, &keydefs)?;
            offset = new_offset;
            current_idx = new_idx;
            if key.kind != KeyKind::None {
                keys.push(key)
            }
        }
        Ok(Self { keys })
    }
    fn build_key(
        current_idx: usize,
        offset: usize,
        keydefs: &[KeyDef],
    ) -> Result<(Key, usize /*new_idx*/, usize /*new_offset*/)> {
        let keydef = keydefs[current_idx];
        match keydef.kind {
            KeyDefKind::VariantId => {
                if current_idx == 0 {
                    Ok((Key::new(offset, KeyKind::None), current_idx + 1, offset + 1))
                } else {
                    Err(format_error!("VariantId cannot be in the middle of keys"))
                }
            }
            KeyDefKind::Padding => Ok((
                Key::new(offset, KeyKind::None),
                current_idx + 1,
                offset + keydef.size,
            )),
            KeyDefKind::ContentAddress(nb_base) => Ok((
                Key::new(offset, KeyKind::ContentAddress(nb_base)),
                current_idx + 1,
                offset + (nb_base as usize + 1) * 4,
            )),
            KeyDefKind::UnsignedInt => Ok((
                Key::new(offset, KeyKind::UnsignedInt(keydef.size)),
                current_idx + 1,
                offset + keydef.size,
            )),
            KeyDefKind::SignedInt => Ok((
                Key::new(offset, KeyKind::SignedInt(keydef.size)),
                current_idx + 1,
                offset + keydef.size,
            )),
            KeyDefKind::CharArray => Ok((
                Key::new(offset, KeyKind::CharArray(keydef.size)),
                current_idx + 1,
                offset + keydef.size,
            )),
            KeyDefKind::PString(flookup, keystoreid) => {
                let (subkey, new_idx, new_offset) = if flookup {
                    let subkey = Self::build_key(current_idx + 1, offset + keydef.size, keydefs)?;
                    let subkey_size = if let KeyKind::CharArray(s) = subkey.0.kind {
                        s
                    } else {
                        return Err(format_error!(
                            "Lookup PString key must be followed by a CharArray key."
                        ));
                    };
                    (Some(subkey_size), subkey.1, subkey.2)
                } else {
                    (None, current_idx + 1, offset + keydef.size)
                };
                Ok((
                    Key::new(
                        offset,
                        KeyKind::PString(keydef.size, keystoreid.into(), subkey),
                    ),
                    new_idx,
                    new_offset,
                ))
            }
        }
    }
}

#[derive(Debug)]
pub struct EntryDef {
    pub variants: Vec<VariantDef>,
    pub size: Size,
}

impl Producable for EntryDef {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<EntryDef> {
        let entry_size = stream.read_u16()? as usize;
        let variant_count = Count::<u8>::produce(stream)?;
        let key_count = Count::<u8>::produce(stream)?;
        let mut variants = Vec::new();
        let mut entry_def = Vec::new();
        let mut current_size = 0;
        for _ in 0..key_count.0 {
            let key = KeyDef::produce(stream)?;
            if key.kind == KeyDefKind::VariantId && !entry_def.is_empty() {
                return Err(format_error!(
                    "VariantId cannot appear in the middle of a entry.",
                    stream
                ));
            }
            current_size += key.size;
            entry_def.push(key);
            match current_size.cmp(&entry_size) {
                Ordering::Greater => {
                    return Err(format_error!(
                        &format!(
                            "Sum of key size ({}) cannot exceed the entry size ({})",
                            current_size, entry_size
                        ),
                        stream
                    ))
                }
                Ordering::Equal => {
                    variants.push(VariantDef::new(entry_def)?);
                    entry_def = Vec::new();
                    current_size = 0;
                }
                Ordering::Less => {
                    /* Noting to do */
                    continue;
                }
            }
        }
        if !entry_def.is_empty() {
            variants.push(VariantDef::new(entry_def)?);
        }
        if variants.len() != variant_count.0 as usize {
            return Err(format_error!(
                &format!(
                    "Entry declare ({}) variants but keys define ({})",
                    variant_count.0,
                    variants.len()
                ),
                stream
            ));
        }
        Ok(EntryDef {
            variants,
            size: Size(entry_size as u64),
        })
    }
}

impl EntryDef {
    pub fn create_entry(&self, reader: &dyn Reader) -> Result<Entry> {
        let variant_id = if self.variants.len() > 1 {
            reader.read_u8(Offset(0))?
        } else {
            0
        };
        let variant_def = &self.variants[variant_id as usize];
        Ok(Entry::new(
            variant_id,
            variant_def,
            reader.create_sub_reader(Offset(0), End::None),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ContentAddress;
    use crate::reader::directory_pack::{Array, KeyDef, KeyDefKind};
    use crate::reader::{Content, Value};

    #[test]
    fn create_entry() {
        let entry_def = EntryDef {
            variants: vec![VariantDef::new(vec![
                KeyDef::new(KeyDefKind::ContentAddress(0), 4),
                KeyDef::new(KeyDefKind::UnsignedInt, 2),
            ])
            .unwrap()],
            size: Size(6),
        };

        {
            let content = vec![0x00, 0x00, 0x00, 0x01, 0x88, 0x99];

            let reader = BufReader::new(content, End::None);
            let entry = entry_def.create_entry(&reader).unwrap();

            assert!(entry.get_variant_id() == 0);
            assert!(
                entry.get_value(0.into()).unwrap()
                    == &Value::Content(Content::new(ContentAddress::new(0.into(), 1.into()), None))
            );
            assert!(entry.get_value(1.into()).unwrap() == &Value::U16(0x8899));
        }

        {
            let content = vec![0x01, 0x00, 0x00, 0x02, 0x66, 0x77];

            let reader = BufReader::new(content, End::None);
            let entry = entry_def.create_entry(&reader).unwrap();

            assert!(entry.get_variant_id() == 0);
            assert!(
                entry.get_value(0.into()).unwrap()
                    == &Value::Content(Content::new(ContentAddress::new(1.into(), 2.into()), None))
            );
            assert!(entry.get_value(1.into()).unwrap() == &Value::U16(0x6677));
        }
    }

    #[test]
    fn create_entry_with_variant() {
        let entry_def = EntryDef {
            variants: vec![
                VariantDef::new(vec![
                    KeyDef::new(KeyDefKind::VariantId, 1),
                    KeyDef::new(KeyDefKind::CharArray, 4),
                    KeyDef::new(KeyDefKind::UnsignedInt, 2),
                ])
                .unwrap(),
                VariantDef::new(vec![
                    KeyDef::new(KeyDefKind::VariantId, 1),
                    KeyDef::new(KeyDefKind::CharArray, 2),
                    KeyDef::new(KeyDefKind::Padding, 1),
                    KeyDef::new(KeyDefKind::SignedInt, 1),
                    KeyDef::new(KeyDefKind::UnsignedInt, 2),
                ])
                .unwrap(),
            ],
            size: Size(7),
        };

        {
            let content = vec![0x00, 0xFF, 0xEE, 0xDD, 0xCC, 0x88, 0x99];

            let reader = BufReader::new(content, End::None);
            let entry = entry_def.create_entry(&reader).unwrap();

            assert!(entry.get_variant_id() == 0);
            assert!(
                entry.get_value(0.into()).unwrap()
                    == &Value::Array(Array::new(vec![0xFF, 0xEE, 0xDD, 0xCC], None))
            );
            assert!(entry.get_value(1.into()).unwrap() == &Value::U16(0x8899));
        }

        {
            let content = vec![0x01, 0xFF, 0xEE, 0xDD, 0xCC, 0x88, 0x99];

            let reader = BufReader::new(content, End::None);
            let entry = entry_def.create_entry(&reader).unwrap();

            assert!(entry.get_variant_id() == 1);
            assert!(
                entry.get_value(0.into()).unwrap()
                    == &Value::Array(Array::new(vec![0xFF, 0xEE], None))
            );
            assert!(entry.get_value(1.into()).unwrap() == &Value::I8(-52));
            assert!(entry.get_value(2.into()).unwrap() == &Value::U16(0x8899));
        }
    }
}
