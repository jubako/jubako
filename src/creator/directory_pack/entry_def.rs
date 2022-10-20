use super::value_store::ValueStore;
use crate::bases::Writable;
use crate::bases::*;
use crate::creator::directory_pack::{Entry, Value};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub enum KeyDef {
    VariantId,
    PString(
        /*flookup_size:*/ usize,
        /*store_handle:*/ Rc<RefCell<ValueStore>>,
    ),
    ContentAddress,
    UnsignedInt(/*max_value:*/ u64),
    Padding(/*size*/ u8),
}

impl KeyDef {
    pub fn new_int() -> Self {
        KeyDef::UnsignedInt(0)
    }

    fn size(&self) -> u16 {
        match self {
            KeyDef::VariantId => 1,
            KeyDef::PString(flookup_size, store_handle) => {
                (*flookup_size as u16) + store_handle.borrow().key_size()
            }
            KeyDef::ContentAddress => 4,
            KeyDef::UnsignedInt(max_value) => needed_bytes(*max_value) as u16,
            KeyDef::Padding(size) => *size as u16,
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
            KeyDef::Padding(_) => 1,
        }
    }
}

impl Writable for KeyDef {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        match self {
            KeyDef::VariantId => stream.write_u8(0b1000_0000),
            KeyDef::PString(flookup_size, store_handle) => {
                let mut flookup_size = *flookup_size;
                let keytype = if flookup_size > 0 {
                    0b0111_0000
                } else {
                    0b0110_0000
                };
                let key_size = (store_handle.borrow().key_size() - 1) as u8;
                let mut written = 0;
                written += stream.write_u8(keytype + key_size)?;
                written += store_handle.borrow().get_idx().write(stream)?;
                if flookup_size > 0 {
                    let keytype: u8 = 0b0100_0000;
                    if flookup_size <= 8 {
                        written += stream.write_u8(keytype + (flookup_size - 1) as u8)?;
                    } else if flookup_size <= 2056 {
                        flookup_size -= 9;
                        written += stream
                            .write_u8(keytype + ((flookup_size >> 8) & 0x03) as u8 + 0b1000)?;
                        written += stream.write_u8(flookup_size as u8)?;
                    } else {
                        panic!()
                    }
                }
                Ok(written)
            }
            KeyDef::ContentAddress => stream.write_u8(0b0001_0000),
            KeyDef::UnsignedInt(max_value) => {
                let size = needed_bytes(*max_value) as u8;
                let key_type = 0b0010_0000;
                stream.write_u8(key_type + (size - 1))
            }
            KeyDef::Padding(size) => {
                let key_type = 0b0000_0000;
                stream.write_u8(key_type + (size - 1))
            }
        }
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

    pub fn write_entry(&self, entry: &Entry, stream: &mut dyn OutStream) -> Result<usize> {
        let mut written = 0;
        if self.need_variant_id {
            written += stream.write_u8(entry.variant_id)?;
        }
        let mut value_iter = entry.values.iter();
        for key in &self.keys {
            match key {
                KeyDef::PString(flookup_size, store_handle) => {
                    let flookup_size = *flookup_size as usize;
                    let value = value_iter.next().unwrap();
                    if let Value::Array { data, value_id } = value {
                        written += stream.write_sized(
                            value_id.unwrap(),
                            store_handle.borrow().key_size() as usize,
                        )?;
                        written += stream.write_data(data)?;
                        // Data is truncate at flookup_size. We just want to write 0 if data is shorter than flookup_size
                        written +=
                            stream.write_data(vec![0; flookup_size - data.len()].as_slice())?;
                    } else {
                        return Err("Not a Array".to_string().into());
                    }
                }
                KeyDef::ContentAddress => {
                    let value = value_iter.next().unwrap();
                    if let Value::Content(value) = value {
                        // [TODO] Write base
                        written += value.content_address.write(stream)?;
                    } else {
                        return Err("Not a Content".to_string().into());
                    }
                }
                KeyDef::UnsignedInt(max_value) => {
                    let value = value_iter.next().unwrap();
                    let size = needed_bytes(*max_value);
                    if let Value::Unsigned(value) = value {
                        written += stream.write_sized(*value, size)?;
                    } else {
                        return Err("Not a unsigned".to_string().into());
                    }
                }
                KeyDef::Padding(size) => {
                    let data = vec![0x00; *size as usize];
                    written += stream.write(&data)?;
                }
                KeyDef::VariantId => unreachable!(),
            }
        }
        Ok(written)
    }

    fn entry_size(&self) -> u16 {
        let base = if self.need_variant_id { 1 } else { 0 };
        self.keys.iter().map(|k| k.size()).sum::<u16>() + base
    }

    fn key_count(&self) -> u8 {
        let base = if self.need_variant_id { 1 } else { 0 };
        self.keys.iter().map(|k| k.key_count()).sum::<u8>() + base
    }

    fn fill_to_size(&mut self, size: u16) {
        let current_size = self.entry_size();
        let mut padding_size = size - current_size;
        while padding_size >= 16 {
            self.keys.push(KeyDef::Padding(16));
            padding_size -= 16;
        }
        if padding_size > 0 {
            self.keys.push(KeyDef::Padding(padding_size as u8))
        }
    }
}

impl Writable for VariantDef {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        if self.need_variant_id {
            written += KeyDef::VariantId.write(stream)?;
        }
        for key in &self.keys {
            written += key.write(stream)?;
        }
        Ok(written)
    }
}

#[derive(Debug)]
pub struct EntryDef {
    pub variants: Vec<VariantDef>,
    entry_size: u16,
}

impl EntryDef {
    pub fn new(variants: Vec<VariantDef>) -> Self {
        let mut ret = Self {
            variants,
            entry_size: 0,
        };
        if ret.variants.len() > 1 {
            for variant in &mut ret.variants {
                variant.need_variant_id = true;
            }
        }
        ret
    }

    pub fn finalize(&mut self) {
        self.entry_size = self.variants.iter().map(|v| v.entry_size()).max().unwrap();
        for variant in &mut self.variants {
            variant.fill_to_size(self.entry_size);
        }
    }

    pub fn write_entry(&self, entry: &Entry, stream: &mut dyn OutStream) -> Result<usize> {
        let variant_def = &self.variants[entry.variant_id as usize];
        let written = variant_def.write_entry(entry, stream)?;
        assert_eq!(written, self.entry_size as usize);
        Ok(written)
    }

    pub fn entry_size(&self) -> u16 {
        self.entry_size
    }

    fn key_count(&self) -> u8 {
        self.variants.iter().map(|v| v.key_count()).sum()
    }
}

impl Writable for EntryDef {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += stream.write_u16(self.entry_size())?;
        written += stream.write_u8(self.variants.len() as u8)?;
        written += stream.write_u8(self.key_count())?;
        for variant in &self.variants {
            written += variant.write(stream)?;
        }
        Ok(written)
    }
}
