use super::ContentAddress;
use crate::bases::*;

#[derive(PartialEq, Eq, Debug)]
pub enum Value {
    Content(ContentAddress),
    Unsigned(Word<u64>),
    Signed(Word<i64>),
    Array(Vec<u8>),
}
