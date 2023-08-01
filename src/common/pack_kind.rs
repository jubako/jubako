use crate::bases::*;
use std::fmt::Debug;

const JBK_MAGIC: u32 = u32::from_be_bytes(*b"\0jbk");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackKind {
    Manifest,
    Directory,
    Content,
    Container,
}

impl Producable for PackKind {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        let kind = flux.read_u8()?;
        match kind {
            b'm' => Ok(PackKind::Manifest),
            b'd' => Ok(PackKind::Directory),
            b'c' => Ok(PackKind::Content),
            b'C' => Ok(PackKind::Container),
            _ => Err(format_error!(&format!("Invalid pack kind {kind}"), flux)),
        }
    }
}
impl SizedProducable for PackKind {
    const SIZE: usize = 1;
}

impl Writable for PackKind {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        match self {
            PackKind::Manifest => stream.write_u8(b'm'),
            PackKind::Directory => stream.write_u8(b'd'),
            PackKind::Content => stream.write_u8(b'c'),
            PackKind::Container => stream.write_u8(b'C'),
        }
    }
}

pub(crate) struct FullPackKind(pub PackKind);

impl Producable for FullPackKind {
    type Output = PackKind;
    fn produce(flux: &mut Flux) -> Result<Self::Output> {
        let magic = flux.read_u32()?;
        if (magic >> 8) != JBK_MAGIC {
            Err(format_error!("Not a JBK kind", flux))
        } else {
            match (magic & 0xFF) as u8 {
                b'm' => Ok(PackKind::Manifest),
                b'd' => Ok(PackKind::Directory),
                b'c' => Ok(PackKind::Content),
                b'C' => Ok(PackKind::Container),
                _ => Err(format_error!("Invalid pack kind", flux)),
            }
        }
    }
}

impl SizedProducable for FullPackKind {
    const SIZE: usize = 4;
}

impl Writable for FullPackKind {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_all(b"jbk")?;
        self.0.write(stream)
    }
}
