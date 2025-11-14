use super::ContentAddress;
use crate::bases::*;

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum Value {
    Content(ContentAddress),
    Unsigned(u64),
    Signed(i64),
    Array(SmallBytes),
}
