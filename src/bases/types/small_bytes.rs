use std::{borrow::Borrow, ops::Deref};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct SmallBytes(smallvec::SmallVec<[u8; 2 * size_of::<usize>()]>);

impl SmallBytes {
    pub fn new() -> Self {
        Self(smallvec::SmallVec::new())
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.0.reserve(additional)
    }

    pub fn push(&mut self, value: u8) {
        self.0.push(value)
    }

    pub fn extend_from_slice(&mut self, slice: &[u8]) {
        self.0.extend_from_slice(slice);
    }
}

impl From<Vec<u8>> for SmallBytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value.into())
    }
}

impl From<&[u8]> for SmallBytes {
    fn from(value: &[u8]) -> Self {
        Self(value.into())
    }
}

impl From<&str> for SmallBytes {
    fn from(value: &str) -> Self {
        Self(value.as_bytes().into())
    }
}

impl<const N: usize> From<&[u8; N]> for SmallBytes {
    fn from(value: &[u8; N]) -> Self {
        Self(value.as_slice().into())
    }
}

impl Deref for SmallBytes {
    type Target = [u8];
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl AsRef<[u8]> for SmallBytes {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl Borrow<[u8]> for SmallBytes {
    #[inline]
    fn borrow(&self) -> &[u8] {
        self
    }
}

impl PartialEq<&[u8]> for SmallBytes {
    fn eq(&self, other: &&[u8]) -> bool {
        self.0.as_slice() == *other
    }
}

impl<const N: usize> PartialEq<&[u8; N]> for SmallBytes {
    fn eq(&self, other: &&[u8; N]) -> bool {
        self.0.as_slice() == other.as_slice()
    }
}

impl From<SmallBytes> for Box<[u8]> {
    fn from(value: SmallBytes) -> Self {
        value.0.into_boxed_slice()
    }
}
