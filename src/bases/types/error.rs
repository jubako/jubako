use crate::bases::*;

#[cfg(debug_assertions)]
use std::backtrace::Backtrace;
use std::fmt;
use std::ops::Deref;
use std::string::FromUtf8Error;

#[cfg(feature = "lzma")]
use xz2::stream::Error as lzmaError;

#[derive(Debug)]
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
/// Kind of error returned by Jubako.
pub enum ErrorKind {
    /// Io error. Can be raised by any error on the underlying system.
    Io(std::io::Error),

    /// Corruption of the file detected (internal crc doesn't match)
    Corrupted(Vec<u8>, [u8; 4]),

    /// Format error detected.
    ///
    /// Crc is valid but data is not.
    /// This can be because of a bug or (badly) forged file
    Format(FormatError),

    /// Library cannot read the version of the file
    Version(u8, u8),

    /// This is not a Jubako file
    NotAJbk,

    /// Arg given to the function/method is not valid (out of bound, ...)
    Arg(String),

    /// Type of the given value (at creation) doesn't correspond to the property type.
    ///
    /// This almost always because of a bug in the calling code.
    /// This could, and maybe will, be replaced by assert.
    WrongType(String),

    /// Something in the archive cannot be read because Jubako has not be compile with
    /// the right feature.
    MissingFeature {
        feature_name: String,
        msg: String,
    },
    NotFound(String),
}

pub struct Error {
    pub error: ErrorKind,
    #[cfg(debug_assertions)]
    bt: Backtrace,
}

impl Deref for Error {
    type Target = ErrorKind;
    fn deref(&self) -> &Self::Target {
        &self.error
    }
}

impl From<ErrorKind> for Error {
    fn from(value: ErrorKind) -> Self {
        Self::new(value)
    }
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

    pub fn arg(msg: impl ToString) -> Error {
        Error::new(ErrorKind::Arg(msg.to_string()))
    }

    pub fn version_error(major: u8, minor: u8) -> Error {
        Error::new(ErrorKind::Version(major, minor))
    }

    pub fn wrong_type(msg: impl Into<String>) -> Self {
        Error::new(ErrorKind::WrongType(msg.into()))
    }
    pub fn missfeature(feature_name: impl Into<String>, msg: impl Into<String>) -> Self {
        Error::new(ErrorKind::MissingFeature {
            feature_name: feature_name.into(),
            msg: msg.into(),
        })
    }

    pub fn notfound(msg: impl Into<String>) -> Self {
        Error::new(ErrorKind::NotFound(msg.into()))
    }

    pub fn corrupted(buf: Vec<u8>, found_crc: [u8; 4]) -> Self {
        Error::new(ErrorKind::Corrupted(buf, found_crc))
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

#[cfg(feature = "explorable")]
impl From<Error> for graphex::Error {
    fn from(value: Error) -> Self {
        graphex::Error::Other(Box::new(value))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.error {
            ErrorKind::Io(e) => write!(f, "IO Error {e}"),
            ErrorKind::Corrupted(buf, found_crc) => write!(
                f,
                "Not a valid checksum : {buf:X?}. Found is {found_crc:X?}"
            ),
            ErrorKind::Format(e) => write!(f, "Jubako format error {e}"),
            ErrorKind::Version(major, minor) => {
                writeln!(f, "Jubako version error. Found ({major},{minor})")?;
                writeln!(f, "Jubako specification is still unstable and compatibility is not guarenteed yet.")?;
                writeln!(f, "Open this container with a older version of your tool.")?;
                write!(
                    f,
                    "You may open a issue on `https://github.com/jubako/jubako` if you are lost."
                )
            }
            ErrorKind::NotAJbk => write!(f, "This is not a Jubako archive"),
            ErrorKind::Arg(msg) => write!(f, "Invalid argument: {msg}"),
            ErrorKind::WrongType(msg) => write!(f, "Wrong type: {msg}"),
            ErrorKind::MissingFeature { feature_name, msg } => {
                writeln!(f, "{msg}")?;
                writeln!(
                    f,
                    "You may want to reinstall you tool with feature {feature_name}"
                )
            }
            ErrorKind::NotFound(msg) => writeln!(f, "{msg}"),
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
    #[cfg(feature = "nightly")]
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
