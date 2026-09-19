use crate::{creator::EntryTrait, PropertyCount, PropertyName, Value, VariantName};
use std::collections::HashMap;

pub struct SimpleEntry<PN: PropertyName, VN: VariantName> {
    variant_name: Option<VN>,
    values: HashMap<PN, Value>,
}

impl<PN: PropertyName, VN: VariantName> SimpleEntry<PN, VN> {
    pub fn new(variant_name: VN, values: impl Into<HashMap<PN, Value>>) -> Self {
        Self {
            variant_name: Some(variant_name),
            values: values.into(),
        }
    }
}

impl<PN: PropertyName, VN: VariantName> From<HashMap<PN, Value>> for SimpleEntry<PN, VN> {
    fn from(values: HashMap<PN, Value>) -> Self {
        Self {
            variant_name: None,
            values,
        }
    }
}

impl<PN: PropertyName, VN: VariantName> EntryTrait<PN, VN> for SimpleEntry<PN, VN> {
    fn variant_name(&self) -> Option<VN> {
        self.variant_name
    }

    fn value(&self, name: &PN) -> Value {
        self.values[name].clone()
    }

    fn value_count(&self) -> PropertyCount {
        PropertyCount::from(self.values.len() as u8)
    }
}
