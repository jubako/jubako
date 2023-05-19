use super::super::raw_layout::{PropertyKind, RawProperty};
use super::property::Property;
use crate::bases::*;
use std::sync::Arc;

#[derive(Debug)]
pub struct Properties(Box<[Property]>);

pub type SharedProperties = Arc<[Property]>;

impl std::ops::Deref for Properties {
    type Target = [Property];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<Properties> for SharedProperties {
    fn from(props: Properties) -> Self {
        props.0.into()
    }
}

impl Properties {
    pub fn new(initial_offset: usize, raw_properties: Vec<RawProperty>) -> Result<Self> {
        let mut offset = initial_offset;
        let mut properties = Vec::new();
        for raw_property in raw_properties {
            let property = Property::new(offset, raw_property.kind);
            offset += raw_property.size;
            if property.kind != PropertyKind::Padding && property.kind != PropertyKind::VariantId {
                properties.push(property)
            }
        }
        Ok(Properties(properties.into_boxed_slice()))
    }
}
