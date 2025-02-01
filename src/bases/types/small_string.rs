use std::{borrow::Borrow, fmt::Display, ops::Deref, str::Utf8Error};

use crate::SmallBytes;

/// A SmallVec which is guaranty to be a value utf8 content.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct SmallString(SmallBytes);

impl SmallString {
    pub fn as_str(&self) -> &str {
        // SAFETY: Bytes in SmallVec is guaranted to be valid utf8 from
        // SmallString contructors and the absence of way to modify this SmallVec.
        unsafe { std::str::from_utf8_unchecked(&self.0) }
    }
}

impl SmallString {
    pub fn from_byte_vec(buf: SmallBytes) -> Result<Self, Utf8Error> {
        match std::str::from_utf8(buf.as_ref()) {
            Ok(_) => Ok(Self(buf)),
            Err(e) => Err(e),
        }
    }

    pub fn from_byte_slice(buf: &[u8]) -> Result<Self, Utf8Error> {
        match std::str::from_utf8(buf) {
            Ok(_) => Ok(Self(buf.into())),
            Err(e) => Err(e),
        }
    }
}

impl std::hash::Hash for SmallString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl From<&str> for SmallString {
    fn from(value: &str) -> Self {
        Self(SmallBytes::from(value.as_bytes()))
    }
}

impl TryFrom<SmallBytes> for SmallString {
    type Error = Utf8Error;
    fn try_from(value: SmallBytes) -> Result<Self, Self::Error> {
        Self::from_byte_vec(value)
    }
}

impl TryFrom<&[u8]> for SmallString {
    type Error = Utf8Error;
    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Self::from_byte_slice(value)
    }
}

impl<const N: usize> TryFrom<&[u8; N]> for SmallString {
    type Error = Utf8Error;
    fn try_from(value: &[u8; N]) -> Result<Self, Self::Error> {
        Self::from_byte_slice(value.as_slice())
    }
}

impl Deref for SmallString {
    type Target = str;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for SmallString {
    #[inline]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for SmallString {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Display for SmallString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(feature = "explorable_serde")]
impl serde::Serialize for SmallString {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use std::hash::DefaultHasher;

    use super::*;
    #[test]
    fn byte_string_ok() {
        assert_eq!(SmallString::from("Hello").as_ref(), "Hello");
        assert_eq!(SmallString::from("Héllü").as_ref(), "Héllü");
        assert_eq!(SmallString::from("Héllü\x57").as_ref(), "HéllüW");
    }

    #[test]
    fn byte_string_ko() {
        assert!(SmallString::try_from(b"Hello\xFF").is_err());
    }

    #[test]
    fn hash_equal() {
        use std::hash::{Hash, Hasher};
        let b_hash = {
            let b = SmallString::from("Hello");
            let mut s = DefaultHasher::new();
            b.hash(&mut s);
            s.finish()
        };
        let s_hash = {
            let mut s = DefaultHasher::new();
            "Hello".hash(&mut s);
            s.finish()
        };
        assert_eq!(b_hash, s_hash);
    }
}
