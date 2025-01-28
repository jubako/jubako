use crate::bases::*;

#[cfg(debug_assertions)]
use std::backtrace::Backtrace;

use std::fmt;
use std::ops::Deref;
use std::{str::Utf8Error, string::FromUtf8Error};

use thiserror::Error;
#[cfg(feature = "lzma")]
use xz2::stream::Error as lzmaError;

#[derive(Error, Debug)]
pub struct FormatError {
    what: String,
    where_: Option<Offset>,
}

impl FormatError {
    pub(crate) fn new(what: impl Into<String>, where_: Option<Offset>) -> Self {
        FormatError {
            what: what.into(),
            where_,
        }
    }
}

//#[macro_export]
macro_rules! format_error {
    ($what:expr, $stream:ident) => {
        crate::bases::FormatError::new($what, Some($stream.global_offset())).into()
    };
    ($what:expr) => {
        crate::bases::FormatError::new($what, None).into()
    };
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.where_ {
            None => write!(f, "{}", self.what),
            Some(w) => write!(f, "{} at offset {}", self.what, w),
        }
    }
}

#[derive(Error, Debug)]
#[error("Not a valid checksum : {buf:X?}. Found is {found_checksum:X?}")]
pub struct CorruptedFile {
    pub buf: Vec<u8>,
    pub found_checksum: [u8; 4],
}

#[derive(Error, Debug)]
#[error(
    "Jubako version error. Found ({major},{minor})
         Jubako specification is still unstable and compatibility is not guarenteed yet.
         Open this container with a older version of your tool.
         You may open a issue on `https://github.com/jubako/jubako` if you are lost."
)]
pub struct VersionError {
    pub major: u8,
    pub minor: u8,
}

#[derive(Error, Debug)]
#[error(
    "{msg}
         You may want to reinstall you tool with feature {name}"
)]
pub struct MissingFeatureError {
    pub name: &'static str,
    pub msg: &'static str,
}

#[derive(Error, Debug)]
/// Kind of error returned by Jubako.
pub enum ErrorKind {
    /// Io error. Can be raised by any error on the underlying system.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Corruption of the file detected (internal crc doesn't match)
    #[error(transparent)]
    Corrupted(#[from] CorruptedFile),

    /// Format error detected.
    ///
    /// Crc is valid but data is not.
    /// This can be because of a bug or (badly) forged file
    #[error("Jubako format error {0}")]
    Format(#[from] FormatError),

    /// Library cannot read the version of the file
    #[error(transparent)]
    Version(#[from] VersionError),

    /// This is not a Jubako file
    #[error("This is not a Jubako archive")]
    NotAJbk,

    /// Something in the archive cannot be read because Jubako has not be compile with
    /// the right feature.
    #[error(transparent)]
    MissingFeature(#[from] MissingFeatureError),
}

#[derive(Error, Debug)]
#[error("{source}")]
#[cfg_attr(not(debug_assertions), repr(transparent))]
pub struct Error {
    #[source]
    source: ErrorKind,
    #[cfg(debug_assertions)]
    backtrace: Option<Backtrace>,
}

impl From<ErrorKind> for Error {
    #[cfg(not(debug_assertions))]
    fn from(source: ErrorKind) -> Self {
        Self { source }
    }

    #[cfg(debug_assertions)]
    fn from(source: ErrorKind) -> Self {
        let backtrace = std::backtrace::Backtrace::capture();
        match backtrace.status() {
            std::backtrace::BacktraceStatus::Disabled
            | std::backtrace::BacktraceStatus::Unsupported => Self {
                source,
                backtrace: None,
            },
            _ => Self {
                source,
                backtrace: Some(backtrace),
            },
        }
    }
}

impl Deref for Error {
    type Target = ErrorKind;
    fn deref(&self) -> &Self::Target {
        &self.source
    }
}

macro_rules! impl_from_error {
    ($what:ty) => {
        impl From<$what> for Error {
            fn from(e: $what) -> Error {
                ErrorKind::from(e).into()
            }
        }
    };
}

impl_from_error!(std::io::Error);
impl_from_error!(FormatError);
impl_from_error!(VersionError);
impl_from_error!(MissingFeatureError);
impl_from_error!(CorruptedFile);

impl From<FromUtf8Error> for Error {
    fn from(_e: FromUtf8Error) -> Error {
        FormatError::new("Utf8DecodingError", None).into()
    }
}

impl From<Utf8Error> for Error {
    fn from(_e: Utf8Error) -> Error {
        FormatError::new("Utf8DecodingError", None).into()
    }
}

#[cfg(feature = "lzma")]
impl From<lzmaError> for Error {
    fn from(_e: lzmaError) -> Error {
        FormatError::new("Lzma compression error", None).into()
    }
}

#[cfg(feature = "explorable")]
impl From<Error> for graphex::Error {
    fn from(value: Error) -> Self {
        graphex::Error::Other(Box::new(value))
    }
}

pub type Result<T> = std::result::Result<T, Error>;
