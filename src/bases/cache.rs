use super::types::*;
use std::sync::{Arc, OnceLock};

pub trait CachableSource<Value> {
    type Idx: Into<usize> + Copy;
    fn get_len(&self) -> usize;
    fn get_value(&self, id: Self::Idx) -> Result<Arc<Value>>;
}

pub struct VecCache<Value, Source>
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
        let value = self.values[index.into()].get_or_try_init(|| self.source.get_value(index))?;
        Ok(value)
    }
}
