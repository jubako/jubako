use crate::bases::*;
use std::os::unix::ffi::OsStringExt;

#[derive(PartialEq, Eq, Debug)]
pub enum PackPos {
    Offset(Offset),
    Path(Vec<u8>),
}

impl From<std::path::PathBuf> for PackPos {
    fn from(p: std::path::PathBuf) -> Self {
        PackPos::Path(p.as_os_str().to_os_string().into_vec())
    }
}
