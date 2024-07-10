use super::types::*;
use std::sync::{Arc, OnceLock};

pub(crate) trait CachableSource<Value> {
    type Idx: Into<usize> + Copy;
    fn get_len(&self) -> usize;
    fn get_value(&self, id: Self::Idx) -> Result<Arc<Value>>;
}

pub(crate) struct VecCache<Value, Source>
where
    Source: CachableSource<Value>,
{
    source: Arc<Source>,
    values: Vec<OnceLock<Arc<Value>>>,
}

impl<Value, Source> VecCache<Value, Source>
where
    Source: CachableSource<Value>,
{
    pub fn new(source: Arc<Source>) -> Self {
        let mut values = Vec::new();
        values.resize_with(source.get_len(), Default::default);
        Self { source, values }
    }

    pub fn get(&self, index: Source::Idx) -> Result<&Arc<Value>> {
        let cache_slot = &self.values[index.into()];
        if cache_slot.get().is_none() {
            let new_value = self.source.get_value(index)?;
            let _ = cache_slot.set(new_value);
        }

        Ok(cache_slot.get().unwrap())
    }
}
