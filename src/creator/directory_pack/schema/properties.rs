use super::super::{layout, EntryTrait};
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

    pub fn finalize(&self, variant_name: Option<String>) -> layout::Properties {
        let variant = variant_name.map(layout::Property::VariantId);
        variant
            .into_iter()
            .chain(self.0.iter().map(|p| p.finalize()))
            .collect()
    }

    pub fn process(&mut self, entry: &dyn EntryTrait) {
        self.0.iter_mut().for_each(|p| p.process(entry))
    }
}
