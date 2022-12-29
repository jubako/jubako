use super::ContentAddress;

#[derive(PartialEq, Eq, Debug)]
pub enum Value {
    Content(ContentAddress),
    Unsigned(u64),
    Signed(i64),
    Array(Vec<u8>),
}
