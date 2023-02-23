use super::builder::{AnyBuilder, BuilderTrait};
use super::entry_store::EntryStore;
use super::private::ValueStorageTrait;
use super::ValueStorage;
use crate::bases::*;
use std::rc::Rc;

pub trait SchemaTrait {
    type Builder: BuilderTrait;
    type ValueStorage: ValueStorageTrait;

    fn create_builder(
        &self,
        store: Rc<EntryStore>,
        value_storage: &Self::ValueStorage,
    ) -> Result<Rc<Self::Builder>>;
}

pub struct AnySchema {}

impl SchemaTrait for AnySchema {
    type Builder = AnyBuilder;
    type ValueStorage = ValueStorage;

    fn create_builder(
        &self,
        store: Rc<EntryStore>,
        value_storage: &ValueStorage,
    ) -> Result<Rc<AnyBuilder>> {
        Ok(Rc::new(AnyBuilder::new(store, value_storage)?))
    }
}
