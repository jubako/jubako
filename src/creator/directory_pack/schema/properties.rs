use super::super::{layout, PropertyName};
use super::property::Property;

#[derive(Debug)]
#[repr(transparent)]
pub struct Properties<PN: PropertyName>(Vec<Property<PN>>);
pub type CommonProperties<PN> = Properties<PN>;

#[derive(Debug)]
#[repr(transparent)]
pub struct VariantProperties<PN: PropertyName>(Vec<Property<PN>>);

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

    pub(crate) fn finalize(self, variant_name: Option<&'static str>) -> layout::Properties<PN> {
        let variant = variant_name.map(layout::Property::VariantId);
        variant
            .into_iter()
            .chain(self.0.into_iter().map(|p| p.finalize()))
            .collect()
    }
}
