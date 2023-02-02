use crate::bases::*;

use std::any::Demand;
use std::backtrace::Backtrace;
use std::fmt;
use std::string::FromUtf8Error;

#[cfg(feature = "lzma")]
use lzma::LzmaError;

#[derive(Debug)]
pub struct FormatError {
    what: String,
    where_: Option<Offset>,
}

impl FormatError {
    pub fn new(what: &str, where_: Option<Offset>) -> Self {
        FormatError {
            what: what.into(),
            where_,
        }
    }
}

//#[macro_export]
macro_rules! format_error {
    ($what:expr, $stream:ident) => {
        FormatError::new($what, Some($stream.global_offset())).into()
    };
    ($what:expr) => {
        FormatError::new($what, None).into()
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
pub enum ErrorKind {
    Io(std::io::Error),
    Format(FormatError),
    NotAJbk,
    Arg,
    Other(String),
    OtherStatic(&'static str),
}

pub struct Error {
    pub error: ErrorKind,
    bt: Backtrace,
}

impl Error {
    pub fn new(error: ErrorKind) -> Error {
        Error {
            error,
            bt: Backtrace::capture(),
        }
    }
    pub fn new_arg() -> Error {
        Error::new(ErrorKind::Arg)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::new(ErrorKind::Io(e))
    }
}

impl From<FormatError> for Error {
    fn from(e: FormatError) -> Error {
        Error::new(ErrorKind::Format(e))
    }
}

impl From<FromUtf8Error> for Error {
    fn from(_e: FromUtf8Error) -> Error {
        FormatError::new("Utf8DecodingError", None).into()
    }
}

#[cfg(feature = "lzma")]
impl From<lzma::LzmaError> for Error {
    fn from(e: LzmaError) -> Error {
        match e {
            LzmaError::Io(e) => Error::new(ErrorKind::Io(e)),
            _ => FormatError::new("Lzma compression error", None).into(),
        }
    }
}

impl From<String> for Error {
    fn from(e: String) -> Error {
        Error::new(ErrorKind::Other(e))
    }
}

impl From<&'static str> for Error {
    fn from(e: &'static str) -> Error {
        Error::new(ErrorKind::OtherStatic(e))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.error {
            ErrorKind::Io(e) => write!(f, "IO Error {e}"),
            ErrorKind::Format(e) => write!(f, "Jubako format error {e}"),
            ErrorKind::NotAJbk => write!(f, "This is not a Jubako archive"),
            ErrorKind::Arg => write!(f, "Invalid argument"),
            ErrorKind::Other(e) => write!(f, "Unknown error : {e}"),
            ErrorKind::OtherStatic(e) => write!(f, "Unknown error : {e}"),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Kind: {:?}", self.error)?;
        writeln!(f, "BT: {}", self.bt)?;
        Ok(())
    }
}

impl std::error::Error for Error {
    fn provide<'a>(&'a self, demand: &mut Demand<'a>) {
        demand.provide_ref::<Backtrace>(&self.bt);
        match &self.error {
            ErrorKind::Io(e) => demand.provide_ref::<std::io::Error>(e),
            ErrorKind::Format(e) => demand.provide_ref::<FormatError>(e),
            ErrorKind::Other(e) => demand.provide_ref::<String>(e),
            _ => demand, /* Nothing*/
        };
    }
}

pub type Result<T> = std::result::Result<T, Error>;
