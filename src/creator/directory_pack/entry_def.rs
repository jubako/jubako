use crate::bases::Writable;
use crate::bases::*;
use crate::creator::directory_pack::{Entry, KeyStore};
use crate::creator::Value;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub enum KeyDef {
    VariantId,
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
            KeyDef::VariantId => 1,
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
            KeyDef::VariantId => 1,
            KeyDef::ContentAddress => 1,
            KeyDef::UnsignedInt(_) => 1,
        }
    }
}

impl Writable for KeyDef {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        match self {
            KeyDef::VariantId => {
                stream.write_u8(0b1000_0000)?;
            }
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
    pub(self) need_variant_id: bool,
    pub keys: Vec<KeyDef>,
}

impl VariantDef {
    pub fn new(keys: Vec<KeyDef>) -> Self {
        Self {
            need_variant_id: false,
            keys,
        }
    }

    pub fn write_entry(&self, entry: &Entry, stream: &mut dyn OutStream) -> Result<()> {
        if self.need_variant_id {
            stream.write_u8(entry.variant_id)?;
        }
        let mut value_iter = entry.values.iter();
        for key in &self.keys {
            match key {
                KeyDef::PString(flookup_size, store_handle) => {
                    let flookup_size = *flookup_size as usize;
                    let value = value_iter.next().unwrap();
                    if let Value::Array { data, key_id } = value {
                        stream.write_sized(
                            key_id.unwrap(),
                            store_handle.borrow().key_size() as usize,
                        )?;
                        stream.write_all(data)?;
                        stream.write_all(vec![0; flookup_size - data.len()].as_slice())?;
                    } else {
                        return Err("Not a Array".to_string().into());
                    }
                }
                KeyDef::ContentAddress => {
                    let value = value_iter.next().unwrap();
                    if let Value::Content(value) = value {
                        value.write(stream)?;
                    } else {
                        return Err("Not a Content".to_string().into());
                    }
                }
                KeyDef::UnsignedInt(size) => {
                    let value = value_iter.next().unwrap();
                    if let Value::Unsigned(value) = value {
                        stream.write_sized(*value, *size as usize)?;
                    } else {
                        return Err("Not a unsigned".to_string().into());
                    }
                }
                KeyDef::VariantId => unreachable!(),
            }
        }
        Ok(())
    }

    fn entry_size(&self) -> u16 {
        let base = if self.need_variant_id { 1 } else { 0 };
        self.keys.iter().map(|k| k.size()).sum::<u16>() + base
    }

    fn key_count(&self) -> u8 {
        let base = if self.need_variant_id { 1 } else { 0 };
        self.keys.iter().map(|k| k.key_count()).sum::<u8>() + base
    }
}

impl Writable for VariantDef {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<()> {
        if self.need_variant_id {
            KeyDef::VariantId.write(stream)?;
        }
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
        let mut ret = Self { variants };
        if ret.variants.len() > 1 {
            for variant in &mut ret.variants {
                variant.need_variant_id = true;
            }
        }
        ret
    }

    pub fn write_entry(&self, entry: &Entry, stream: &mut dyn OutStream) -> Result<()> {
        let variant_def = &self.variants[entry.variant_id as usize];
        variant_def.write_entry(entry, stream)
    }

    pub fn entry_size(&self) -> u16 {
        self.variants.iter().map(|v| v.entry_size()).max().unwrap()
    }

    fn key_count(&self) -> u8 {
        self.variants.iter().map(|v| v.key_count()).sum()
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
