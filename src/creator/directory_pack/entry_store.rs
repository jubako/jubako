use super::entry_def;
use super::{Entry, Value, WritableTell};
use crate::bases::*;
use crate::common;
use std::cmp;

pub struct EntryStore {
    idx: Idx<u32>,
    entries: Vec<Entry>,
    entry_def: entry_def::EntryDef,
}

impl EntryStore {
    pub fn new(idx: Idx<u32>, entry_def: entry_def::EntryDef) -> Self {
        Self {
            idx,
            entries: vec![],
            entry_def,
        }
    }

    pub fn add_entry(&mut self, variant_id: u8, values: Vec<common::Value>) {
        self.entries.push(Entry::new(variant_id, values));
    }

    pub fn get_idx(&self) -> Idx<u32> {
        self.idx
    }

    pub(crate) fn finalize(&mut self) {
        for entry in &mut self.entries {
            let mut value_iter = entry.values.iter_mut();
            let variant = &mut self.entry_def.variants[entry.variant_id as usize];
            for key in &mut variant.keys {
                match key {
                    entry_def::KeyDef::PString(flookup_size, store_handle) => {
                        let flookup_size = *flookup_size;
                        let value = value_iter.next().unwrap();
                        if let Value::Array { data, value_id } = value {
                            let to_store = data.split_off(cmp::min(flookup_size, data.len()));
                            *value_id = Some(store_handle.borrow_mut().add_value(&to_store));
                        }
                    }
                    entry_def::KeyDef::UnsignedInt(max_value) => {
                        let value = value_iter.next().unwrap();
                        if let Value::Unsigned(v) = value {
                            *key = entry_def::KeyDef::UnsignedInt(cmp::max(*max_value, *v));
                        }
                    }
                    _ => {
                        value_iter.next();
                    }
                }
            }
        }
        self.entry_def.finalize();
    }
}

impl WritableTell for EntryStore {
    fn write_data(&self, stream: &mut dyn OutStream) -> Result<()> {
        for entry in &self.entries {
            self.entry_def.write_entry(entry, stream)?;
        }
        Ok(())
    }

    fn write_tail(&self, stream: &mut dyn OutStream) -> Result<()> {
        stream.write_u8(0x00)?; // kind
        self.entry_def.write(stream)?;
        stream.write_u64((self.entries.len() * self.entry_def.entry_size() as usize) as u64)?;
        Ok(())
    }
}
