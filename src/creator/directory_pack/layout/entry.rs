use super::properties::{CommonProperties, Properties, VariantProperties};
use crate::bases::Writable;
use crate::bases::*;
use crate::creator::directory_pack::Entry as RawEntry;

#[derive(Debug)]
pub struct Entry {
    pub common: Properties,
    pub variants: Vec<Properties>,
    entry_size: u16,
}

impl Entry {
    pub fn new(common: CommonProperties, variants: Vec<VariantProperties>) -> Self {
        Self {
            common,
            variants: variants.into_iter().map(Properties::from).collect(),
            entry_size: 0,
        }
    }

    pub fn finalize(&mut self) {
        self.entry_size = self.common.entry_size();
        if !self.variants.is_empty() {
            let max_variant_size = self.variants.iter().map(|v| v.entry_size()).max().unwrap();
            self.entry_size += max_variant_size;
            for variant in &mut self.variants {
                variant.fill_to_size(max_variant_size);
            }
        }
    }

    pub fn write_entry(&self, entry: &RawEntry, stream: &mut dyn OutStream) -> Result<usize> {
        assert!(self.variants.is_empty() == entry.variant_id.is_none());
        let written = if self.variants.is_empty() {
            Properties::write_entry(self.common.iter(), entry, stream)?
        } else {
            let mut keys = self
                .common
                .iter()
                .chain(self.variants[entry.variant_id.unwrap() as usize].iter());
            Properties::write_entry(&mut keys, entry, stream)?
        };
        assert_eq!(written, self.entry_size as usize);
        Ok(written)
    }

    pub fn entry_size(&self) -> u16 {
        self.entry_size
    }

    fn key_count(&self) -> u8 {
        self.common.key_count() + self.variants.iter().map(|v| v.key_count()).sum::<u8>()
    }
}

impl Writable for Entry {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += stream.write_u16(self.entry_size())?;
        written += stream.write_u8(self.variants.len() as u8)?;
        written += stream.write_u8(self.key_count())?;
        written += self.common.write(stream)?;
        for variant in &self.variants {
            written += variant.write(stream)?;
        }
        Ok(written)
    }
}
