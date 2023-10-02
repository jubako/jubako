use crate::bases::*;
use std::fmt::Debug;

const JBK_MAGIC: [u8; 3] = *b"jbk";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PackKind {
    Manifest = b'm',
    Directory = b'd',
    Content = b'c',
    Container = b'C',
}

impl Producable for PackKind {
    type Output = Self;
    fn produce(flux: &mut Flux) -> Result<Self> {
        match flux.read_u8()? {
            b'm' => Ok(PackKind::Manifest),
            b'd' => Ok(PackKind::Directory),
            b'c' => Ok(PackKind::Content),
            b'C' => Ok(PackKind::Container),
            kind => Err(format_error!(&format!("Invalid pack kind {kind}"), flux)),
        }
    }
}
impl SizedProducable for PackKind {
    const SIZE: usize = 1;
}

impl Writable for PackKind {
    fn write(&self, stream: &mut dyn OutStream) -> IoResult<usize> {
        stream.write_u8(*self as u8)
    }
}

pub(crate) struct FullPackKind(pub PackKind);

impl Producable for FullPackKind {
    type Output = PackKind;
    fn produce(flux: &mut Flux) -> Result<Self::Output> {
        let mut magic = [0; 3];
        flux.read_exact(&mut magic)?;
        if magic != JBK_MAGIC {
            Err(format_error!("Not a JBK kind", flux))
        } else {
            match flux.read_u8()? {
                b'm' => Ok(PackKind::Manifest),
                b'd' => Ok(PackKind::Directory),
                b'c' => Ok(PackKind::Content),
                b'C' => Ok(PackKind::Container),
                kind => Err(format_error!(&format!("Invalid pack kind {kind}"), flux)),
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
