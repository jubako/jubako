use super::super::{layout, EntryTrait, PropertyName, VariantName};
use super::property::Property;

#[derive(Debug)]
pub struct Properties<PN: PropertyName>(Vec<Property<PN>>);
pub type CommonProperties<PN> = Properties<PN>;

#[derive(Debug)]
pub struct VariantProperties<PN: PropertyName>(pub Vec<Property<PN>>);

impl<PN: PropertyName> std::ops::Deref for Properties<PN> {
    type Target = [Property<PN>];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<PN: PropertyName> std::ops::DerefMut for Properties<PN> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<PN: PropertyName> VariantProperties<PN> {
    pub fn new(keys: Vec<Property<PN>>) -> Self {
        Self(keys)
    }
}

impl<PN: PropertyName> From<VariantProperties<PN>> for Properties<PN> {
    fn from(other: VariantProperties<PN>) -> Self {
        Self(other.0)
    }
}

impl<PN: PropertyName> Properties<PN> {
    pub fn new(keys: Vec<Property<PN>>) -> Self {
        Self(keys)
    }

    pub fn finalize(self, variant_name: Option<String>) -> layout::Properties<PN> {
        let variant = variant_name.map(layout::Property::VariantId);
        variant
            .into_iter()
            .chain(self.0.into_iter().map(|p| p.finalize()))
            .collect()
    }

    pub fn process<VN: VariantName>(&mut self, entry: &dyn EntryTrait<PN, VN>) {
        self.0.iter_mut().for_each(|p| p.process(entry))
    }
}
