use super::entry::Entry;
use super::key::{Key, KeyKind};
use super::key_def::{KeyDef, KeyDefKind};
use crate::bases::*;

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
                    Err(Error::FormatError)
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
                        return Err(Error::FormatError);
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
                return Err(Error::FormatError);
            }
            current_size += key.size;
            entry_def.push(key);
            if current_size > entry_size {
                return Err(Error::FormatError);
            } else if current_size == entry_size {
                variants.push(VariantDef::new(entry_def)?);
                entry_def = Vec::new();
                current_size = 0;
            }
        }
        if !entry_def.is_empty() {
            variants.push(VariantDef::new(entry_def)?);
        }
        if variants.len() != variant_count.0 as usize {
            return Err(Error::FormatError);
        }
        Ok(EntryDef {
            variants,
            size: Size(entry_size as u64),
        })
    }
}

impl EntryDef {
    pub fn create_entry(&self, reader: &dyn Reader) -> Result<Entry> {
        let mut offset = Offset(0);
        let variant_id = if self.variants.len() > 1 {
            offset += 1;
            reader.read_u8(Offset(0))?
        } else {
            0
        };
        let variant_def = &self.variants[variant_id as usize];
        Ok(Entry::new(
            variant_id,
            variant_def,
            reader.create_sub_reader(offset, End::None),
        ))
    }
}
