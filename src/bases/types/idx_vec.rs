use std::{marker::PhantomData, ops::Deref};

pub(crate) trait Index {
    fn into_usize(self) -> usize;
}

#[derive(Debug)]
pub(crate) struct IdxVec<Idx: Index, V> {
    data: Vec<V>,
    _marker: PhantomData<Idx>,
}

impl<Idx, V> IdxVec<Idx, V>
where
    Idx: Index,
{
    fn new() -> Self {
        Vec::new().into()
    }

    fn with_capacity(count: impl Index) -> Self {
        Vec::with_capacity(count.into_usize()).into()
    }
}

impl<Idx, V> Deref for IdxVec<Idx, V>
where
    Idx: Index,
{
    type Target = Vec<V>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<Idx, V> From<Vec<V>> for IdxVec<Idx, V>
where
    Idx: Index,
{
    fn from(value: Vec<V>) -> Self {
        Self {
            data: value,
            _marker: Default::default(),
        }
    }
}

impl<Idx, V> std::ops::Index<Idx> for IdxVec<Idx, V>
where
    Idx: Index,
{
    type Output = V;
    fn index(&self, index: Idx) -> &Self::Output {
        &self.data[index.into_usize()]
    }
}
