use crate::bases::Writable;
use crate::bases::*;
use crate::creator;
use crate::creator::directory_pack::{Entry, KeyStore};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub enum KeyDef {
    PString(
        /*flookup_size:*/ usize,
        /*store_handle:*/ Rc<RefCell<KeyStore>>,
    ),
    ContentAddress,
    UnsignedInt(/*size:*/ u8),
}

impl KeyDef {
    fn size(&self) -> u16 {
        match self {
            KeyDef::PString(flookup_size, store_handle) => {
                (*flookup_size as u16) + store_handle.borrow().key_size()
            }
            KeyDef::ContentAddress => 4,
            KeyDef::UnsignedInt(size) => *size as u16,
        }
    }

    fn key_count(&self) -> u8 {
        match self {
            KeyDef::PString(flookup_size, _) => {
                if *flookup_size > 0 {
                    2
                } else {
                    1
                }
            }
            KeyDef::ContentAddress => 1,
            KeyDef::UnsignedInt(_) => 1,
        }
    }
}

impl Writable for KeyDef {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        match self {
            KeyDef::PString(flookup_size, store_handle) => {
                let keytype = if *flookup_size > 0 {
                    0b0111_0000
                } else {
                    0b0110_0000
                };
                let key_size = (store_handle.borrow().key_size() - 1) as u8;
                stream.write_u8(keytype + key_size)?;
                store_handle.borrow().get_idx().write(stream)?;
            }
            KeyDef::ContentAddress => {
                stream.write_u8(0b0001_0000)?;
            }
            KeyDef::UnsignedInt(size) => {
                let key_type = 0b0010_0000;
                stream.write_u8(key_type + (size - 1))?;
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct VariantDef {
    pub keys: Vec<KeyDef>,
}

impl VariantDef {
    pub fn new(keys: Vec<KeyDef>) -> Self {
        Self { keys }
    }

    pub fn write_entry(&self, entry: &Entry, stream: &mut dyn OutStream) -> Result<()> {
        let mut value_iter = entry.values.iter();
        for key in &self.keys {
            match key {
                KeyDef::PString(flookup_size, store_handle) => {
                    let flookup_size = *flookup_size as usize;
                    let value = value_iter.next().unwrap();
                    if let creator::Value::Array { data, key_id } = value {
                        stream.write_sized(
                            key_id.unwrap(),
                            store_handle.borrow().key_size() as usize,
                        )?;
                        stream.write_all(data)?;
                        stream.write_all(&vec![0; flookup_size - data.len()].as_slice())?;
                    } else {
                        return Err(Error::Other("Not a Array".to_string()));
                    }
                }
                KeyDef::ContentAddress => {
                    let value = value_iter.next().unwrap();
                    if let creator::Value::Content(value) = value {
                        value.write(stream)?;
                    } else {
                        return Err(Error::Other("Not a Content".to_string()));
                    }
                }
                KeyDef::UnsignedInt(size) => {
                    let value = value_iter.next().unwrap();
                    if let creator::Value::Unsigned(value) = value {
                        stream.write_sized(*value, *size as usize);
                    } else {
                        return Err(Error::Other("Not a unsigned".to_string()));
                    }
                }
            }
        }
        Ok(())
    }

    fn entry_size(&self) -> u16 {
        self.keys.iter().map(|k| k.size()).sum()
    }

    fn key_count(&self, with_variant_id: bool) -> u8 {
        let base = if with_variant_id { 1 } else { 0 };
        let key_count: u8 = self.keys.iter().map(|k| k.key_count()).sum();
        key_count + base
    }
}

impl Writable for VariantDef {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        for key in &self.keys {
            key.write(stream)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct EntryDef {
    pub variants: Vec<VariantDef>,
}

impl EntryDef {
    pub fn new(variants: Vec<VariantDef>) -> Self {
        Self { variants }
    }

    pub fn write_entry(&self, entry: &Entry, stream: &mut dyn OutStream) -> Result<()> {
        let variant_id = if self.variants.len() > 1 {
            stream.write_u8(entry.variant_id)?;
            entry.variant_id
        } else {
            0
        };
        let variant_def = &self.variants[variant_id as usize];
        variant_def.write_entry(entry, stream)
    }

    pub fn entry_size(&self) -> u16 {
        let size = self.variants.iter().map(|v| v.entry_size()).max().unwrap();
        // Add the extra key for the variant id
        if self.variants.len() > 1 {
            size + 1
        } else {
            size
        }
    }

    fn key_count(&self) -> u8 {
        let with_variant_id = self.variants.len() > 1;
        self.variants
            .iter()
            .map(|v| v.key_count(with_variant_id))
            .sum()
    }
}

impl Writable for EntryDef {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        stream.write_u16(self.entry_size())?;
        stream.write_u8(self.variants.len() as u8)?;
        stream.write_u8(self.key_count())?;
        for variant in &self.variants {
            variant.write(stream)?;
        }
        Ok(())
    }
}