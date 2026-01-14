use crate::common::{
    ContainerPackHeader, ContentInfo, ContentPackHeader, DirectoryPackHeader, ManifestPackHeader,
    PackHeader, PackInfo, PackKind, PackLocator,
};
use crate::reader::{directory_pack, PackOffsetsIter, ValueStore};
use crate::{self as jbk};
use crate::{bases::CorruptedFile, Offset};
use jbk::bases::*;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
pub struct CheckError {
    pub error: CorruptedFile,
    pub offset: Offset,
    pub size: Size,
    pub structure: &'static str,
}

impl std::fmt::Display for CheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failing block ({}) {:X?} at {}",
            self.structure, self.error.buf, self.offset
        )
    }
}

pub enum CheckResult<T> {
    Ok(T),
    Invalid(Vec<CheckError>),
}

impl<T> CheckResult<T> {
    pub fn is_ok(&self) -> bool {
        matches!(self, CheckResult::Ok(_))
    }
}

//Early_corrupted_return => ec_return
macro_rules! ec_return {
    ($e:expr) => {
        if let CheckResult::Invalid(error) = $e {
            return Ok(CheckResult::Invalid(error));
        }
    };
}

macro_rules! bcheck {
    (@read, $reader:expr, $kind:ty, $offset:expr, $n:expr) => {
        bcheck!(@expr,
            $reader.parse_block_at::<$kind>($offset),
            $reader.global_offset() + $offset,
            Size::from(<$kind>::BLOCK_SIZE),
            $n
        )
    };
    (@array, $reader:expr, $kind:ty, $offset:expr, $count:expr, $n:expr) => {
        bcheck!(@expr,
            ArrayReader::<$kind, _>::new_memory_from_reader($reader, $offset, $count),
            $reader.global_offset() + $offset,
            Size::from(<$kind>::SIZE)*u64::from($count) + 4,
            $n
        )
    };
    (@tail, $reader:expr, $kind:ty, $sized_offset:expr, $n:expr) => {
        bcheck!(@expr,
            $reader.parse_block_in::<<$kind as DataBlockParsable>::TailParser>($sized_offset.offset, $sized_offset.size),
            $reader.global_offset() + $sized_offset.offset,
            Size::from($sized_offset.size) + 4,
            $n
        )
    };
    (@data, $reader:expr, $offset:expr, $size:expr, $n: expr) => {
        bcheck!(@expr,
            $reader.cut_check($offset, $size, crate::bases::BlockCheck::Crc32),
            $reader.global_offset() + $offset,
            Size::from($size) + 4,
            $n
        )
    };

    (@expr, $expr:expr, $offset:expr, $size:expr, $n:expr) => {{
       //println!("check_ser {}", $n);
       match $expr {
           Ok(v) => v,
           Err(e) => match e.try_into() {
                Ok(corrupted) => {
                    println!("early return corrupted");
                    return Ok(CheckResult::Invalid(vec![CheckError {
                        error: corrupted,
                        offset: $offset,
                        size: $size,
                        structure: $n
                    }]))
                }
                Err(e) => return Err(e),
           }
       }
    }};
}

trait CheckBlock {
    fn check_blocks(self, reader: &Reader) -> jbk::Result<CheckResult<()>>;
}

impl CheckBlock for PackHeader {
    fn check_blocks(self, reader: &Reader) -> crate::Result<CheckResult<()>> {
        println!("Checking {:?} pack, {}", self.magic, self.file_size);
        let check_info_pos = self.check_info_pos;
        let check_info_size = self.check_info_size();
        let file_size = self.file_size;
        ec_return!(match self.magic {
            PackKind::Container => {
                let container_header = bcheck!(@read,
                    reader,
                    ContainerPackHeader,
                    Offset::from(PackHeader::BLOCK_SIZE), "ContainerPackHeader"
                );
                container_header.check_blocks(reader)
            }
            PackKind::Manifest => {
                let maniferst_header = bcheck!(@read,
                    reader,
                    ManifestPackHeader,
                    Offset::from(PackHeader::BLOCK_SIZE),
                    "ManifestPackHeader"
                );
                (self, maniferst_header).check_blocks(reader)
            }
            PackKind::Directory => {
                let directory_header = bcheck!(@read,
                    reader,
                    DirectoryPackHeader,
                    Offset::from(PackHeader::BLOCK_SIZE),
                    "DirectoryPackHeader"
                );
                directory_header.check_blocks(reader)
            }
            PackKind::Content => {
                let content_header = bcheck!(@read,
                    reader,
                    ContentPackHeader,
                    Offset::from(PackHeader::BLOCK_SIZE),
                    "ContentPackHeader"
                );
                content_header.check_blocks(reader)
            }
        }?);
        bcheck!(@data, reader, check_info_pos, Size::from(check_info_size), "CheckInfo");
        let mut header_bytes = vec![];
        reader
            .create_stream(Offset::zero(), Size::from(Self::BLOCK_SIZE), true)?
            .read_to_end(&mut header_bytes)?;
        let tail_offset = Offset::from(file_size - Self::BLOCK_SIZE.into());
        let tail_size = Size::from(Self::BLOCK_SIZE);
        let mut tail_bytes = vec![];
        reader
            .create_stream(tail_offset, tail_size, true)?
            .read_to_end(&mut tail_bytes)?;
        tail_bytes.reverse();
        if header_bytes != tail_bytes {
            Ok(CheckResult::Invalid(vec![CheckError {
                error: CorruptedFile {
                    buf: tail_bytes,
                    found_checksum: [0xFF; 4],
                },
                offset: tail_offset,
                size: tail_size,
                structure: "PackTail",
            }]))
        } else {
            Ok(CheckResult::Ok(()))
        }
    }
}

impl CheckBlock for ContainerPackHeader {
    fn check_blocks(self, reader: &Reader) -> crate::Result<CheckResult<()>> {
        let mut pack_offset = self.pack_locators_pos;
        for _idx in self.pack_count {
            let pack_locator = bcheck!(@read,reader, PackLocator, pack_offset, "PackLocator");
            pack_offset += PackLocator::BLOCK_SIZE;
            let pack_reader = reader.cut(pack_locator.pack_pos, pack_locator.pack_size, false)?;
            let pack_header = bcheck!(@read, pack_reader, PackHeader, Offset::zero(), "PackHeader");
            ec_return!(pack_header.check_blocks(&pack_reader)?)
        }
        Ok(CheckResult::Ok(()))
    }
}

impl CheckBlock for (PackHeader, ManifestPackHeader) {
    fn check_blocks(self, reader: &Reader) -> crate::Result<CheckResult<()>> {
        let (pack_header, manifest_header) = self;
        let pack_offsets =
            PackOffsetsIter::new(pack_header.check_info_pos, manifest_header.pack_count);
        for pack_offset in pack_offsets {
            let pack_info = bcheck!(@read, reader, PackInfo, pack_offset, "PackInfo");
            bcheck!(@data, reader, pack_info.check_info_pos.offset , Size::from(pack_info.check_info_pos.size)-Size::new(4), "CheckInfo2");
        }
        if !manifest_header.value_store_posinfo.is_zero() {
            let (_, data_size) = bcheck!(@tail, reader, ValueStore, manifest_header.value_store_posinfo, "ValueStoreHeader");
            bcheck!(@data, reader, manifest_header.value_store_posinfo.offset - data_size - ASize::from(4), data_size, "ValueStoreData");
        }
        Ok(CheckResult::Ok(()))
    }
}
impl CheckBlock for DirectoryPackHeader {
    fn check_blocks(self, reader: &Reader) -> crate::Result<CheckResult<()>> {
        let array_reader = bcheck!(@array, &reader, SizedOffset, self.value_store_ptr_pos, *self.value_store_count, "ValueStoreArray");
        for idx in self.value_store_count {
            let sized_offset = array_reader.index(*idx)?;
            let (_, data_size) =
                bcheck!(@tail, reader, ValueStore, sized_offset, "ValueStoreHeader");
            bcheck!(@data, reader, sized_offset.offset - data_size - ASize::from(4), data_size, "ValueStoreData");
        }
        let array_reader = bcheck!(@array, &reader, SizedOffset, self.entry_store_ptr_pos, *self.entry_store_count, "EntryStoreArray");
        for idx in self.entry_store_count {
            let sized_offset = array_reader.index(*idx)?;
            let layout = bcheck!(@tail, reader, directory_pack::EntryStore, sized_offset, "EntryStoreHeader");
            if layout.is_entry_checked {
                let mut data_offset =
                    sized_offset.offset - layout.entry_count * (layout.entry_size + 4).into();
                for _ in layout.entry_count {
                    bcheck!(@data, reader, data_offset, Size::from(layout.entry_size), "EntryData");
                    data_offset += layout.entry_size + 4;
                }
            } else {
                let data_size = layout.entry_count * layout.entry_size.into();
                bcheck!(@data, reader, sized_offset.offset - data_size - ASize::from(4), data_size, "EntryStoreData");
            }
        }
        let array_reader = bcheck!(@array, &reader, SizedOffset, self.index_ptr_pos, *self.index_count, "IndexArray");
        for idx in self.index_count {
            let sized_offset = array_reader.index(*idx)?;
            bcheck!(@data, reader, sized_offset.offset, Size::from(sized_offset.size), "IndexHeader");
        }
        Ok(CheckResult::Ok(()))
    }
}

impl CheckBlock for ContentPackHeader {
    fn check_blocks(self, reader: &Reader) -> crate::Result<CheckResult<()>> {
        bcheck!(@array, &reader, ContentInfo, self.content_ptr_pos, *self.content_count, "ContentInfoArray");
        let array_reader = bcheck!(@array, &reader, SizedOffset, self.cluster_ptr_pos, *self.cluster_count, "ClusterArray");
        for idx in self.cluster_count {
            let sized_offset = array_reader.index(*idx)?;
            bcheck!(@data, reader, sized_offset.offset, Size::from(sized_offset.size), "ClusterHeader");
        }
        Ok(CheckResult::Ok(()))
    }
}

pub fn check_blocks(path: impl AsRef<Path>) -> jbk::Result<CheckResult<()>> {
    let reader = Reader::from(FileSource::open(path)?);
    let pack_header = bcheck!(@read, reader, jbk::common::PackHeader, Offset::zero(), "PackHeader");
    pack_header.check_blocks(&reader)
}
