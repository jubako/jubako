use crate::bases::Result;
type DynExplorable = Box<dyn Explorable>;

pub trait Explorable: erased_serde::Serialize {
    fn explore_one(&self, _item: &str) -> Result<Option<DynExplorable>> {
        Ok(None)
    }
    fn explore<'item>(
        &self,
        item: &'item str,
    ) -> Result<(Option<DynExplorable>, Option<&'item str>)> {
        if let Some((first, left)) = item.split_once("::") {
            self.explore_one(first).map(|explo| (explo, Some(left)))
        } else {
            self.explore_one(item).map(|explo| (explo, None))
        }
    }
}

impl Explorable for Vec<u8> {}
impl Explorable for String {}

erased_serde::serialize_trait_object!(Explorable);
