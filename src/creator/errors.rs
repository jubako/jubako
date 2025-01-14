use std::fmt;
use std::string::FromUtf16Error;

#[derive(Debug)]
/// Error type used on creation side.
pub enum Error {
    /// Io error. Can be raised by any error on the underlying system.
    Io(std::io::Error),

    UTF16(FromUtf16Error),

    /// Type of the given value (at creation) doesn't correspond to the property type.
    ///
    /// This almost always because of a bug in the calling code.
    /// This could, and maybe will, be replaced by assert.
    WrongType(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<FromUtf16Error> for Error {
    fn from(value: FromUtf16Error) -> Self {
        Error::UTF16(value)
    }
}

impl Error {
    pub fn wrong_type(msg: impl Into<String>) -> Self {
        Error::WrongType(msg.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => writeln!(f, "IO error {e}"),
            Error::UTF16(e) => writeln!(f, "{e}"),
            Error::WrongType(e) => writeln!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {
    #[cfg(feature = "nightly")]
    fn provide<'a>(&'a self, request: &mut std::error::Request<'a>) {
        #[cfg(debug_assertions)]
        request.provide_ref::<Backtrace>(&self.bt);
        match &self.error {
            Error::Io(e) => request.provide_ref::<std::io::Error>(e),
            Error::UTF16(e) => request.provide_ref::<std::io::Error>(e),
            Error::WrongType(e) => request.provide_ref::<std::io::Error>(e),
            _ => request, /* Nothing*/
        };
    }
}
