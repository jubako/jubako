mod entry;
mod properties;
mod property;

pub use entry::Entry;
pub use properties::{CommonProperties, VariantProperties};
pub use property::Property;

use super::{Value, ValueStore};
