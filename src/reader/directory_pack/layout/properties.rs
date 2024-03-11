use super::super::raw_layout::{PropertyKind, RawProperty};
use super::property::Property;
use crate::bases::*;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, serde::Serialize)]
pub struct Properties(HashMap<String, Property>);

pub type SharedProperties = Arc<Properties>;

impl std::ops::Deref for Properties {
    type Target = HashMap<String, Property>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Properties {
    pub fn new(initial_offset: usize, raw_properties: Vec<RawProperty>) -> Result<Self> {
        let mut offset = initial_offset;
        let mut properties = HashMap::new();
        for raw_property in raw_properties {
            let property = Property::new(offset, raw_property.kind);
            offset += raw_property.size;
            if property.kind != PropertyKind::Padding && property.kind != PropertyKind::VariantId {
                properties.insert(raw_property.name.unwrap(), property);
            }
        }
        Ok(Properties(properties))
    }
}
