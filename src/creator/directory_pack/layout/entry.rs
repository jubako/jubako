use super::properties::Properties;
use crate::bases::Writable;
use crate::bases::*;
use crate::creator::directory_pack::EntryTrait;

#[derive(Debug)]
pub struct Entry {
    pub common: Properties,
    pub variants: Vec<Properties>,
    pub entry_size: u16,
}

impl Entry {
    pub fn write_entry(&self, entry: &dyn EntryTrait, stream: &mut dyn OutStream) -> Result<usize> {
        assert!(self.variants.is_empty() == entry.variant_id().is_none());
        let written = if self.variants.is_empty() {
            Properties::write_entry(self.common.iter(), entry, stream)?
        } else {
            let mut keys = self
                .common
                .iter()
                .chain(self.variants[entry.variant_id().unwrap().into_usize()].iter());
            Properties::write_entry(&mut keys, entry, stream)?
        };
        assert_eq!(written, self.entry_size as usize);
        Ok(written)
    }

    fn key_count(&self) -> Count<u8> {
        (self.common.key_count() + self.variants.iter().map(|v| v.key_count()).sum::<u8>()).into()
    }
}

impl Writable for Entry {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        let mut written = 0;
        written += stream.write_u16(self.entry_size)?;
        written += stream.write_u8(self.variants.len() as u8)?;
        written += self.key_count().write(stream)?;
        written += self.common.write(stream)?;
        for variant in &self.variants {
            written += variant.write(stream)?;
        }
        Ok(written)
    }
}
