use super::builder::{AnyBuilder, BuilderTrait};
use super::layout::Layout;
use crate::bases::*;

pub trait SchemaTrait {
    type Builder: BuilderTrait;
    fn check_layout(&self, layout: &Layout) -> Result<Self::Builder>;
}

pub struct AnySchema {}

impl SchemaTrait for AnySchema {
    type Builder = AnyBuilder;
    fn check_layout(&self, layout: &Layout) -> Result<AnyBuilder> {
        Ok(AnyBuilder::new_from_layout(layout))
    }
}
