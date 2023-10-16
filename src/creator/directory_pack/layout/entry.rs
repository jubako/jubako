use super::super::{PropertyName, VariantName};
use super::properties::Properties;
use crate::bases::Writable;
use crate::bases::*;
use crate::creator::directory_pack::EntryTrait;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Entry<PN: PropertyName, VN: VariantName> {
    pub common: Properties<PN>,
    pub variants: Vec<Properties<PN>>,
    pub variants_map: HashMap<VN, VariantIdx>,
    pub entry_size: u16,
}

impl<PN: PropertyName, VN: VariantName> Entry<PN, VN> {
    pub fn write_entry(
        &self,
        entry: &dyn EntryTrait<PN, VN>,
        stream: &mut dyn OutStream,
    ) -> Result<usize> {
        assert!(self.variants.is_empty() == entry.variant_name().is_none());
        let written = if self.variants.is_empty() {
            Properties::write_entry(self.common.iter(), None, entry, stream)?
        } else {
            let variant_id = self.variants_map[&entry.variant_name().unwrap()];
            let mut keys = self
                .common
                .iter()
                .chain(self.variants[variant_id.into_usize()].iter());
            Properties::write_entry(&mut keys, Some(variant_id), entry, stream)?
        };
        assert_eq!(written, self.entry_size as usize);
        Ok(written)
    }

    fn key_count(&self) -> Count<u8> {
        (self.common.len() as u8 + self.variants.iter().map(|v| v.len() as u8).sum::<u8>()).into()
    }
}

impl<PN: PropertyName, VN: VariantName> Writable for Entry<PN, VN> {
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
