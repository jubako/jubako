use crate::bases::*;

#[cfg(debug_assertions)]
use std::backtrace::Backtrace;
use std::fmt;
use std::string::FromUtf8Error;

#[cfg(feature = "lzma")]
use xz2::stream::Error as lzmaError;

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
    OtherError(Box<dyn std::error::Error + Send + Sync>),
    Other(String),
    OtherStatic(&'static str),
}

pub struct Error {
    pub error: ErrorKind,
    #[cfg(debug_assertions)]
    bt: Backtrace,
}

impl Error {
    #[cfg(debug_assertions)]
    pub fn new(error: ErrorKind) -> Error {
        Error {
            error,
            bt: Backtrace::capture(),
        }
    }

    #[cfg(not(debug_assertions))]
    pub fn new(error: ErrorKind) -> Error {
        Error { error }
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
impl From<lzmaError> for Error {
    fn from(_e: lzmaError) -> Error {
        FormatError::new("Lzma compression error", None).into()
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

impl From<Box<dyn std::error::Error + Sync + Send>> for Error {
    fn from(e: Box<dyn std::error::Error + Sync + Send>) -> Error {
        Error::new(ErrorKind::OtherError(e))
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
            ErrorKind::OtherError(e) => write!(f, "Other error : {e}"),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Kind: {:?}", self.error)?;
        #[cfg(debug_assertions)]
        writeln!(f, "BT: {}", self.bt)?;
        Ok(())
    }
}

impl std::error::Error for Error {
    #[cfg(feature = "error_generic_member_access")]
    fn provide<'a>(&'a self, request: &mut std::error::Request<'a>) {
        #[cfg(debug_assertions)]
        request.provide_ref::<Backtrace>(&self.bt);
        match &self.error {
            ErrorKind::Io(e) => request.provide_ref::<std::io::Error>(e),
            ErrorKind::Format(e) => request.provide_ref::<FormatError>(e),
            ErrorKind::Other(e) => request.provide_ref::<String>(e),
            _ => request, /* Nothing*/
        };
    }
}

pub type Result<T> = std::result::Result<T, Error>;
