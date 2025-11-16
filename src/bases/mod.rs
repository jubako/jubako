#[macro_use]
mod types;
mod block;
mod cache;
mod io;
mod parsing;
pub(crate) mod primitive;
mod prop_type;
mod reader;
mod skip;
mod write;

pub(crate) use block::*;
pub(crate) use cache::*;
pub use io::FileSource;
pub(crate) use io::*;
pub(crate) use parsing::*;
pub(crate) use prop_type::*;
pub(crate) use reader::CheckReader;
pub use reader::Reader;
pub(crate) use skip::*;
use std::cmp;
use std::marker::PhantomData;
pub use types::*;
pub(crate) use write::OutStream;
pub(crate) use write::*;

pub(crate) use std::io::Result as IoResult;

/// ArrayReader is a wrapper a reader to access element stored as a array.
/// (Consecutif block of data of the same size).
pub(crate) struct ArrayReader<OutType, IdxType> {
    reader: CheckReader,
    length: Count<IdxType>,
    elem_size: ASize,
    produced_type: PhantomData<OutType>,
}

impl<OutType, IdxType> ArrayReader<OutType, IdxType>
where
    OutType: SizedParsable,
    u64: std::convert::From<Count<IdxType>>,
    IdxType: Copy,
{
    pub fn new_memory_from_reader(
        reader: &Reader,
        at: Offset,
        length: Count<IdxType>,
    ) -> Result<Self> {
        let array_size = Size::new(OutType::SIZE as u64 * u64::from(length));
        let reader = reader.cut_check(at, array_size, BlockCheck::Crc32)?;
        Ok(Self {
            reader,
            length,
            elem_size: OutType::SIZE.into(),
            produced_type: PhantomData,
        })
    }
}

impl<OutType: Parsable, IdxType> IndexTrait<Idx<IdxType>> for ArrayReader<OutType, IdxType>
where
    u64: std::convert::From<IdxType>,
    IdxType: std::cmp::PartialOrd + Copy + std::fmt::Debug,
{
    type OutputType = Result<OutType::Output>;
    fn index(&self, idx: Idx<IdxType>) -> Result<OutType::Output> {
        debug_assert!(
            idx.is_valid(self.length),
            "idx = {:?}, length = {:?}",
            idx,
            self.length
        );
        let offset = u64::from(idx.into_base()) * self.elem_size.into_u64();
        self.reader
            .parse_in::<OutType>(Offset::from(offset), self.elem_size)
    }
}

pub(crate) fn needed_bytes<T>(mut val: T) -> ByteSize
where
    T: std::cmp::PartialOrd + std::ops::Shr<Output = T> + From<u8>,
{
    let mut nb_bytes = 0_usize;
    while val > 0.into() {
        val = val >> 8.into();
        nb_bytes += 1;
    }
    nb_bytes = cmp::max(nb_bytes, 1);
    nb_bytes.try_into().unwrap()
}

pub trait PropertyName: std::cmp::Eq + std::hash::Hash + Copy + Send + 'static {
    fn as_str(&self) -> &'static str;
}

impl PropertyName for &'static str {
    fn as_str(&self) -> &'static str {
        self
    }
}

pub trait VariantName: std::cmp::Eq + std::hash::Hash + Copy + Send {
    fn as_str(&self) -> &'static str;
}
impl VariantName for &'static str {
    fn as_str(&self) -> &'static str {
        self
    }
}
impl VariantName for () {
    fn as_str(&self) -> &'static str {
        ""
    }
}

#[macro_export]
macro_rules! variants {
    ($vname:ident { $($name:ident => $jname:literal),+  $(,)? }) => {
        #[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
        #[repr(u8)]
        pub enum $vname {
            $($name),+
        }

        impl $vname {
            pub(crate) const fn get_str(&self) -> &'static str {
                use $vname::*;
                match self {
                    $($name => $jname),+
                }
            }
                }

        impl jbk::VariantName for $vname {
            fn as_str(&self) -> &'static str {
                self.get_str()
            }
        }

        impl TryFrom<&str>  for $vname {
            type Error = ();
            fn try_from(v: &str) -> Result<Self, ()> {
                $(
                    if v == $jname {return Ok(Self::$name);}
                )+
                Err(())
            }
        }
    };
}

#[macro_export]
macro_rules! properties {
    ($pname:ident { $($name:ident:$kind:literal => $jname:literal),+ $(,)? }) => {
        #[derive(Copy, Clone, Hash, Eq, PartialEq, Debug)]
        pub enum $pname {
            $($name),+
        }

        impl $pname {
            const fn get_str(&self) -> &'static str {
                use $pname::*;
                match self {
                    $($name => $jname),+
                }
            }
            const fn get_type(&self) -> &'static str {
                use $pname::*;
                match self {
                    $($name => $kind),+
                }
            }
        }

        impl jbk::PropertyName for $pname {
            fn as_str(&self) -> &'static str {
                self.get_str()
            }
        }
    };
}

#[macro_export]
macro_rules! layout_builder {
    ($container:ident[common][$key:expr], $value_storage:expr, $error:ident) => {
       $crate::layout_builder!(@from_key, $container.common, $key, $value_storage, $error)
    };
    ($container:ident[$variant:expr][$key:expr], $value_storage:expr, $error:ident) => {{
        let variant = $container.get_variant($variant);
        match variant {
            None => Err($error($crate::concatcp!(
                "Variant `", ($variant.get_str()), "` is not present."
             )))?,
            Some(variant) => $crate::layout_builder!(@from_key, variant, $key, $value_storage, $error)
        }
    }};

    (@from_key, $container:expr, $key:expr, $value_storage:expr, $error:ident) => {
        $container
            .get($key)
            .ok_or($error($crate::concatcp!(
                "Property ", ($key.get_str()), " is not present."
            )))?
            .as_builder($value_storage)?
            .ok_or($error($crate::concatcp!(
                "Property ", ($key.get_str()), " is not a ", ($key.get_type()), " property."
            )))?
    };
}
