use super::ContentAddress;
use crate::bases::*;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Content {
    pub(crate) content_address: ContentAddress,
    base: Option<Box<Content>>,
}

impl Content {
    pub fn new(content_address: ContentAddress, base: Option<Content>) -> Self {
        Self {
            content_address,
            base: base.map(Box::new),
        }
    }

    pub fn pack_id(&self) -> Id<u8> {
        self.content_address.pack_id
    }

    pub fn content_id(&self) -> Idx<u32> {
        self.content_address.content_id
    }
}

impl From<ContentAddress> for Content {
    fn from(other: ContentAddress) -> Self {
        Content::new(other, None)
    }
}

impl From<(Id<u8>, Idx<u32>)> for Content {
    fn from(other: (Id<u8>, Idx<u32>)) -> Self {
        let (pack_id, content_id) = other;
        Content::new(ContentAddress::new(pack_id, content_id), None)
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum Value {
    Content(Content),
    Unsigned(u64),
    Signed(i64),
    Array(Vec<u8>),
}
