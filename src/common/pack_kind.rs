use crate::bases::*;
use std::fmt::Debug;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackKind {
    Manifest,
    Directory,
    Content,
}

impl Producable for PackKind {
    type Output = Self;
    fn produce(stream: &mut dyn Stream) -> Result<Self> {
        match stream.read_u32()? {
            0x6a_62_6b_6d_u32 => Ok(PackKind::Manifest),  // jbkm
            0x6a_62_6b_64_u32 => Ok(PackKind::Directory), // jbkd
            0x6a_62_6b_63_u32 => Ok(PackKind::Content),   // jbkc
            _ => Err(format_error!("Invalid pack kind", stream)),
        }
    }
}

impl Writable for PackKind {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        match self {
            PackKind::Manifest => stream.write_u32(0x6a_62_6b_6d_u32),
            PackKind::Directory => stream.write_u32(0x6a_62_6b_64_u32),
            PackKind::Content => stream.write_u32(0x6a_62_6b_63_u32),
        }
    }
}
