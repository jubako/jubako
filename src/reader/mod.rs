#[macro_use]
mod content_pack;
mod byte_region;
mod byte_slice;
mod byte_stream;
mod container_pack;
mod directory_pack;
mod jubako;
mod locator;
mod manifest_pack;
mod missing;

pub use self::jubako::Container;
pub use container_pack::ContainerPack;
pub use content_pack::ContentPack;
pub use directory_pack::{builder, layout};
pub use missing::MayMissPack;
pub type EntryStore = std::sync::Arc<directory_pack::EntryStore>;
pub(crate) use crate::common::ManifestPackHeader;
pub use crate::common::{ContentAddress, PackInfo};
use crate::{
    bases::SizedBlockParsable,
    common::{PackHeader, PackKind},
    Offset, Reader, Result, Size,
};
pub use byte_region::ByteRegion;
pub use byte_slice::ByteSlice;
pub use byte_stream::ByteStream;
pub use directory_pack::{
    CompareTrait, DirectoryPack, EntryTrait, Index, RangeTrait as Range, RawValue, ValueStorage,
};
pub use layout::VariantPart;
pub use locator::{ChainedLocator, FsLocator, PackLocatorTrait};
pub use manifest_pack::{ManifestPack, PackOffsetsIter};

fn read_pack_header<SpecificPackHeader: SizedBlockParsable>(
    reader: &Reader,
    offset: Offset,
    kind: PackKind,
) -> Result<(PackHeader, SpecificPackHeader::Output)> {
    let reader = reader.cut(
        offset,
        Size::from(PackHeader::BLOCK_SIZE + SpecificPackHeader::BLOCK_SIZE),
        true,
    )?;
    let pack_header = reader.parse_block_at::<PackHeader>(Offset::zero())?;
    if pack_header.magic != kind {
        return Err(format_error!("Pack Magic is not {kind} Pack"));
    }

    let header =
        reader.parse_block_at::<SpecificPackHeader>(Offset::from(PackHeader::BLOCK_SIZE))?;
    Ok((pack_header, header))
}
