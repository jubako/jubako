mod check;
mod compression_type;
mod content_address;
mod content_info;
mod headers;
mod pack_info;
mod pack_kind;
mod pack_locator;
mod value;
use uuid::Uuid;

use crate::bases::*;
pub use check::{CheckInfo, CheckKind, ManifestCheckStream};
pub use compression_type::CompressionType;
pub use content_address::ContentAddress;
pub use content_info::ContentInfo;
pub use headers::*;
pub use pack_info::PackInfo;
pub use pack_kind::PackKind;
pub use pack_locator::PackLocator;
pub use value::Value;

pub(crate) use pack_kind::FullPackKind;

impl Producable for Uuid {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let mut v = [0_u8; 16];
        flux.read_exact(&mut v)?;
        Ok(Uuid::from_bytes(v))
    }
}
impl Writable for Uuid {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_data(self.as_bytes())
    }
}

/// A Pack is the more global entity in Jubako.
/// It is a "File", which can be a single file in the fs
/// or embedded in another file.
pub trait Pack {
    fn kind(&self) -> PackKind;
    fn app_vendor_id(&self) -> u32;
    fn version(&self) -> (u8, u8);
    fn uuid(&self) -> Uuid;
    fn size(&self) -> Size;
    fn check(&self) -> Result<bool>;
}
