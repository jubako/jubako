use super::builder::{AnyBuilder, BuilderTrait};
use super::entry_store::EntryStore;
use crate::bases::*;
use std::rc::Rc;

pub trait SchemaTrait {
    type Builder: BuilderTrait;
    fn create_builder(&self, store: Rc<EntryStore>) -> Result<Self::Builder>;
}

pub struct AnySchema {}

impl SchemaTrait for AnySchema {
    type Builder = AnyBuilder;
    fn create_builder(&self, store: Rc<EntryStore>) -> Result<AnyBuilder> {
        Ok(AnyBuilder::new(store))
    }
}
