use super::super::{PropertyName, VariantName};
use super::properties::Properties;
use crate::bases::Serializable;
use crate::bases::*;
use crate::creator::{ProcessedEntry, Result};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct Entry<PN: PropertyName, VN: VariantName> {
    pub common: Properties<PN>,
    pub variants: Vec<Properties<PN>>,
    pub variants_map: HashMap<VN, VariantIdx>,
    pub entry_size: u16,
}

impl<PN: PropertyName, VN: VariantName> Entry<PN, VN> {
    pub fn serialize_entry(
        &self,
        entry: &ProcessedEntry<VN>,
        ser: &mut Serializer,
    ) -> Result<usize> {
        assert!(self.variants.is_empty() == entry.variant_name.is_none());
        let written = if self.variants.is_empty() {
            Properties::serialize_entry(self.common.iter(), None, &entry.values, ser)?
        } else {
            let variant_id = self.variants_map[&entry.variant_name.unwrap()];
            let mut keys = self
                .common
                .iter()
                .chain(self.variants[variant_id.into_usize()].iter());
            Properties::serialize_entry(&mut keys, Some(variant_id), &entry.values, ser)?
        };
        assert_eq!(written, self.entry_size as usize);
        Ok(written)
    }

    fn key_count(&self) -> Count<u8> {
        (self.common.len() as u8 + self.variants.iter().map(|v| v.len() as u8).sum::<u8>()).into()
    }
}

impl<PN: PropertyName, VN: VariantName> Serializable for Entry<PN, VN> {
    fn serialize(&self, ser: &mut Serializer) -> IoResult<usize> {
        let mut written = 0;
        written += ser.write_u16(self.entry_size)?;
        written += ser.write_u8(self.variants.len() as u8)?;
        written += self.key_count().serialize(ser)?;
        written += self.common.serialize(ser)?;
        for variant in &self.variants {
            written += variant.serialize(ser)?;
        }
        Ok(written)
    }
}
