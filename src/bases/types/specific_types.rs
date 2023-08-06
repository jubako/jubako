use super::{Count, Id, Idx};

trait Next {
    fn next(self) -> Self;
}

pub struct IntoIter<T> {
    current: T,
    end: T,
}

impl<T> std::iter::Iterator for IntoIter<T>
where
    T: std::cmp::PartialEq + Next + Copy,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let ret = self.current;
            self.current = self.current.next();
            Some(ret)
        }
    }
}

macro_rules! to_u64 {
    ( $base: tt, $name: ty ) => {
        impl $name {
            pub fn into_u64(self) -> u64 {
                self.0 .0 as u64
            }
        }

        impl From<$name> for u64 {
            fn from(i: $name) -> u64 {
                i.into_u64()
            }
        }
    };
}

macro_rules! to_u32 {
    ( u64, $name: ty ) => {};
    ( $base: tt, $name: ty ) => {
        impl $name {
            pub fn into_u32(self) -> u32 {
                self.0 .0 as u32
            }
        }

        impl From<$name> for u32 {
            fn from(i: $name) -> u32 {
                i.into_u32()
            }
        }
    };
}

macro_rules! to_u16 {
    ( u64, $name: ty ) => {};
    ( u32, $name: ty ) => {};
    ( $base: tt, $name: ty ) => {
        impl $name {
            pub fn into_u16(self) -> u16 {
                self.0 .0 as u16
            }
        }

        impl From<$name> for u16 {
            fn from(i: $name) -> u16 {
                i.into_u16()
            }
        }
    };
}

macro_rules! to_u8 {
    ( u64, $name: ty ) => {};
    ( u32, $name: ty ) => {};
    ( u16, $name: ty ) => {};
    ( $base: tt, $name: ty ) => {
        impl $name {
            pub fn into_u8(self) -> u8 {
                self.0 .0 as u8
            }
        }

        impl From<$name> for u8 {
            fn from(i: $name) -> u8 {
                i.into_u8()
            }
        }
    };
}

macro_rules! to_usize {
    ( u64 ) => {
        // We can convert a u64 to usize only if we are on 64bits
        #[cfg(target_pointer_width = "64")]
        pub fn into_usize(self) -> usize {
            self.0 .0 as usize
        }
    };
    ( u32 ) => {
        // We can convert a u32 to usize only if we are on 32 or 64bits
        #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
        pub fn into_usize(self) -> usize {
            self.0 .0 as usize
        }
    };
    ( $base: tt ) => {
        // We can convert a u8 and u16 to usize all the time
        pub fn into_usize(self) -> usize {
            self.0 .0 as usize
        }
    };
}

macro_rules! impl_add {
    ( Id, $base:ty, $idx_name:ident, $count_name:ident ) => {};
    ( Idx, $base:ty, $idx_name:ident, $count_name:ident ) => {
        impl std::ops::Add for $idx_name {
            type Output = $idx_name;
            fn add(self, other: $idx_name) -> Self::Output {
                $idx_name(self.0 + other.0)
            }
        }

        impl std::ops::Add<$count_name> for $idx_name {
            type Output = $idx_name;
            fn add(self, other: $count_name) -> Self::Output {
                $idx_name(self.0 + other.0)
            }
        }

        impl std::ops::AddAssign<$base> for $idx_name {
            fn add_assign(&mut self, rhs: $base) {
                self.0 += rhs;
            }
        }

        impl std::ops::AddAssign<$count_name> for $idx_name {
            fn add_assign(&mut self, rhs: $count_name) {
                self.0 += rhs.0 .0;
            }
        }

        impl std::ops::Sub<$base> for $idx_name {
            type Output = $idx_name;
            fn sub(self, other: $base) -> Self::Output {
                $idx_name::from(self.0 .0 - other)
            }
        }

        impl std::ops::Sub for $idx_name {
            type Output = $count_name;
            fn sub(self, other: $idx_name) -> Self::Output {
                $count_name::from(self.0 .0 - other.0 .0)
            }
        }

        impl $idx_name {
            pub fn is_valid(&self, c: $count_name) -> bool {
                self.0.is_valid(*c)
            }
        }
    };
}

macro_rules! def_type {
    ( Id, $base:ty, $idx_name:ident, $count_name:ident ) => {
        #[derive(PartialEq, Eq, Copy, Clone, Hash, Default)]
        pub struct $idx_name(pub Id<$base>);
    };
    ( Idx, $base:ty, $idx_name:ident, $count_name:ident ) => {
        #[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash, Default)]
        pub struct $idx_name(pub Idx<$base>);
    };
}

macro_rules! specific {
    ( $base: tt, $idx_name:ident($inner_idx:ident), $count_name:ident, $base_name: expr ) => {
        // Declare our Index
        def_type! {$inner_idx, $base, $idx_name, $count_name}

        impl $idx_name {
            to_usize!($base);
        }

        to_u64!($base, $idx_name);
        to_u32!($base, $idx_name);
        to_u16!($base, $idx_name);
        to_u8!($base, $idx_name);

        impl Next for $idx_name {
            fn next(self) -> Self {
                (self.0 .0 + 1).into()
            }
        }

        impl From<$base> for $idx_name {
            fn from(idx: $base) -> Self {
                Self($inner_idx::<$base>(idx))
            }
        }

        impl From<$inner_idx<$base>> for $idx_name {
            fn from(idx: $inner_idx<$base>) -> Self {
                Self(idx)
            }
        }

        impl std::ops::Not for $idx_name {
            type Output = bool;
            fn not(self) -> Self::Output {
                self.0 .0 == 0
            }
        }

        impl std::ops::Deref for $idx_name {
            type Target = $inner_idx<$base>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl_add!($inner_idx, $base, $idx_name, $count_name);

        impl std::fmt::Display for $idx_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} {}", $base_name, self.0)
            }
        }

        impl std::fmt::Debug for $idx_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!(
                    "{}<{}> : {}",
                    stringify!($idx_name),
                    stringify!($base),
                    self.0 .0
                ))
            }
        }

        // Declare our Count
        #[derive(PartialEq, Eq, Copy, Clone)]
        pub struct $count_name(pub Count<$base>);

        impl $count_name {
            to_usize!($base);
        }

        to_u64!($base, $count_name);
        to_u32!($base, $count_name);
        to_u16!($base, $count_name);
        to_u8!($base, $count_name);

        impl From<$base> for $count_name {
            fn from(count: $base) -> Self {
                Self(Count::<$base>(count))
            }
        }

        impl From<Count<$base>> for $count_name {
            fn from(count: Count<$base>) -> Self {
                Self(count)
            }
        }

        impl std::ops::Not for $count_name {
            type Output = bool;
            fn not(self) -> Self::Output {
                self.0 .0 == 0
            }
        }

        impl std::ops::Div<$base> for $count_name {
            type Output = Self;
            fn div(self, div: $base) -> Self {
                $count_name::from(self.0 .0 / div)
            }
        }

        impl std::ops::Deref for $count_name {
            type Target = Count<$base>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::Add<$base> for $count_name {
            type Output = $count_name;
            fn add(self, other: $base) -> Self::Output {
                $count_name(self.0 + other)
            }
        }

        impl std::ops::AddAssign<$base> for $count_name {
            fn add_assign(&mut self, rhs: $base) {
                self.0 += rhs;
            }
        }

        impl std::iter::IntoIterator for $count_name {
            type Item = $idx_name;
            type IntoIter = IntoIter<$idx_name>;
            fn into_iter(self) -> Self::IntoIter {
                IntoIter {
                    current: 0.into(),
                    end: self.0 .0.into(),
                }
            }
        }

        impl std::fmt::Display for $count_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{} {}", $base_name, self.0)
            }
        }

        impl std::fmt::Debug for $count_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!(
                    "{}<{}> : {}",
                    stringify!($count_name),
                    stringify!($base),
                    self.0 .0
                ))
            }
        }
    };
}

specific! {u32, EntryIdx(Idx), EntryCount, "Entry"}
specific! {u32, EntryStoreIdx(Idx), EntryStoreCount, "EntryStore"}
specific! {u32, ClusterIdx(Idx), ClusterCount, "Cluster"}
specific! {u32, ContentIdx(Idx), ContentCount, "Content"}
specific! {u32, IndexIdx(Idx), IndexCount, "Index"}
specific! {u16, PackId(Id), PackCount, "Pack"}
specific! {u8,  ValueStoreIdx(Idx), ValueStoreCount, "ValueStore"}
specific! {u16,  BlobIdx(Idx), BlobCount, "Blob"}
specific! {u8,  PropertyIdx(Idx), PropertyCount, "Property"}
specific! {u64,  ValueIdx(Idx), ValueCount, "Value"}
specific! {u8, VariantIdx(Idx), VariantCount, "Variant"}

#[cfg(target_pointer_width = "64")]
impl From<ValueStoreIdx> for usize {
    fn from(v: ValueStoreIdx) -> usize {
        v.into_usize()
    }
}

#[cfg(target_pointer_width = "64")]
impl From<EntryStoreIdx> for usize {
    fn from(v: EntryStoreIdx) -> usize {
        v.into_usize()
    }
}
