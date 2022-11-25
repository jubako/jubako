use super::variant::Variant;
use crate::bases::Writable;
use crate::bases::*;
use crate::creator::directory_pack::Entry as RawEntry;

#[derive(Debug)]
pub struct Entry {
    pub variants: Vec<Variant>,
    entry_size: u16,
}

impl Entry {
    pub fn new(variants: Vec<Variant>) -> Self {
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

    pub fn write_entry(&self, entry: &RawEntry, stream: &mut dyn OutStream) -> Result<usize> {
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

impl Writable for Entry {
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
