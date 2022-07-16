use crate::bases::*;
use lzma::LzmaError;
use std::fmt;
use std::string::FromUtf8Error;

#[derive(Debug)]
pub struct FormatError {
    what: String,
    where_: Option<Offset>,
}

impl FormatError {
    pub fn new(what: &str, where_: Option<Offset>) -> Error {
        FormatError {
            what: what.into(),
            where_,
        }
        .into()
    }
}

//#[macro_export]
macro_rules! format_error {
    ($what:expr, $stream:ident) => {
        FormatError::new($what, Some($stream.global_offset()))
    };
    ($what:expr) => {
        FormatError::new($what, None)
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

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Format(FormatError),
    Arg,
    Other(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<FormatError> for Error {
    fn from(e: FormatError) -> Error {
        Error::Format(e)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_e: FromUtf8Error) -> Error {
        FormatError::new("Utf8DecodingError", None)
    }
}

impl From<lzma::LzmaError> for Error {
    fn from(e: LzmaError) -> Error {
        match e {
            LzmaError::Io(e) => Error::Io(e),
            _ => FormatError::new("Lzma compression error", None),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO Error {}", e),
            Error::Format(e) => write!(f, "Jubako format error {}", e),
            Error::Arg => write!(f, "Invalid argument"),
            Error::Other(e) => write!(f, "Unknown error : {}", e),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
