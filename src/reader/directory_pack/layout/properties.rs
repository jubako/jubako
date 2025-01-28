use super::super::raw_layout::{PropertyKind, RawProperty};
use super::property::Property;
use super::SmallString;
use crate::PropertyName;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
#[cfg_attr(feature = "explorable_serde", derive(serde::Serialize))]
pub struct Properties(HashMap<SmallString, Property>);

pub(crate) type SharedProperties = Arc<Properties>;

impl Properties {
    pub(crate) fn new(initial_offset: usize, raw_properties: Vec<RawProperty>) -> Self {
        let mut offset = initial_offset;
        let mut properties = HashMap::new();
        for raw_property in raw_properties {
            let property = Property::new(offset, raw_property.kind);
            offset += raw_property.size;
            if property.kind != PropertyKind::Padding && property.kind != PropertyKind::VariantId {
                properties.insert(raw_property.name, property);
            }
        }
        Properties(properties)
    }

    pub fn get(&self, name: impl PropertyName) -> Option<&Property> {
        self.0.get(name.as_str())
    }

    pub fn iter(&self) -> impl Iterator<Item = (&SmallString, &Property)> {
        self.0.iter()
    }

    #[cfg(test)]
    pub fn inner(&self) -> &HashMap<SmallString, Property> {
        &self.0
    }
}

#[cfg(feature = "explorable")]
impl graphex::Display for Properties {
    fn print_content(&self, out: &mut graphex::Output) -> graphex::Result {
        let mut keys = self
            .0
            .iter()
            .map(|(k, v)| (v.offset, k))
            .collect::<Vec<_>>();
        keys.sort_unstable();
        for (_, key) in keys.iter() {
            out.field(key, &self.0[*key])?;
        }
        Ok(())
    }
}
