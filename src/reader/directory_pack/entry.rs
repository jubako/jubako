use super::raw_value::RawValue;
use crate::bases::*;

pub trait EntryTrait {
    fn get_variant_id(&self) -> u8;
    fn get_value(&self, idx: Idx<u8>) -> Result<&RawValue>;
}
