use super::super::{layout, Value};
use super::property::Property;

#[derive(Debug)]
pub struct Properties(Vec<Property>);
pub type CommonProperties = Properties;

#[derive(Debug)]
pub struct VariantProperties(pub Vec<Property>);

impl std::ops::Deref for Properties {
    type Target = [Property];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Properties {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl VariantProperties {
    pub fn new(keys: Vec<Property>) -> Self {
        Self(keys)
    }
}

impl From<VariantProperties> for Properties {
    fn from(other: VariantProperties) -> Self {
        Self(other.0)
    }
}

impl Properties {
    pub fn new(keys: Vec<Property>) -> Self {
        Self(keys)
    }

    pub fn finalize(&self, variant: bool) -> layout::Properties {
        let variant = if variant {
            Some(layout::Property::VariantId)
        } else {
            None
        };
        variant
            .into_iter()
            .chain(self.0.iter().map(|p| p.finalize()))
            .collect()
    }

    pub fn process<'a>(&mut self, values: &mut impl Iterator<Item = &'a Value>) {
        self.0.iter_mut().for_each(|p| p.process(values))
    }
}
