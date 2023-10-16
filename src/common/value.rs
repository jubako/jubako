use super::ContentAddress;
use crate::bases::*;

#[derive(PartialEq, Eq, Debug)]
pub enum Value {
    Content(ContentAddress),
    Unsigned(u64),
    Signed(i64),
    UnsignedWord(Word<u64>),
    SignedWord(Word<i64>),
    Array(Vec<u8>),
}
