use crate::bases::*;
use std::fmt::Debug;
use std::io::{self, Read};

#[derive(Clone, Copy)]
pub enum CheckKind {
    None = 0,
    Blake3 = 1,
}

impl Producable for CheckKind {
    type Output = Self;
    fn produce(stream: &mut Stream) -> Result<Self> {
        let kind = stream.read_u8()?;
        match kind {
            0_u8 => Ok(CheckKind::None),
            1_u8 => Ok(CheckKind::Blake3),
            _ => Err(format_error!(
                &format!("Invalid check kind {}", kind),
                stream
            )),
        }
    }
}

impl Writable for CheckKind {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u8(*self as u8)
    }
}

impl Producable for blake3::Hash {
    type Output = Self;
    fn produce(stream: &mut Stream) -> Result<Self> {
        let mut v = [0_u8; blake3::OUT_LEN];
        stream.read_exact(&mut v)?;
        Ok(v.into())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CheckInfo {
    b3hash: Option<blake3::Hash>,
}

impl Producable for CheckInfo {
    type Output = Self;
    fn produce(stream: &mut Stream) -> Result<Self> {
        let kind = CheckKind::produce(stream)?;
        let b3hash = match kind {
            CheckKind::Blake3 => Some(blake3::Hash::produce(stream)?),
            _ => None,
        };
        Ok(Self { b3hash })
    }
}

impl CheckInfo {
    pub fn check(&self, source: &mut dyn Read) -> Result<bool> {
        if let Some(b3hash) = self.b3hash {
            let mut hasher = blake3::Hasher::new();
            io::copy(source, &mut hasher)?;
            let hash = hasher.finalize();
            Ok(hash == b3hash)
        } else {
            Ok(true)
        }
    }
}
