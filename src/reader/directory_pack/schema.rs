use super::builder::{AnyBuilder, BuilderTrait};
use super::entry_store::EntryStore;
use super::private::ValueStorageTrait;
use crate::bases::*;
use std::rc::Rc;

pub trait SchemaTrait {
    type Builder: BuilderTrait;
    fn create_builder<ValueStorage>(
        &self,
        store: Rc<EntryStore>,
        value_storage: &ValueStorage,
    ) -> Result<Rc<Self::Builder>>
    where
        ValueStorage: ValueStorageTrait;
}

pub struct AnySchema {}

impl SchemaTrait for AnySchema {
    type Builder = AnyBuilder;
    fn create_builder<ValueStorage>(
        &self,
        store: Rc<EntryStore>,
        value_storage: &ValueStorage,
    ) -> Result<Rc<AnyBuilder>>
    where
        ValueStorage: ValueStorageTrait,
    {
        Ok(Rc::new(AnyBuilder::new(store, value_storage)?))
    }
}
