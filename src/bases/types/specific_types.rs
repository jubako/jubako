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
    ( $base: ty ) => {
        pub fn into_u64(self) -> u64 {
            self.0 .0 as u64
        }
    };
}

macro_rules! to_u32 {
    ( u64 ) => {};
    ( $base: ty ) => {
        pub fn into_u32(self) -> u32 {
            self.0 .0 as u32
        }
    };
}

macro_rules! to_u16 {
    ( u64 ) => {};
    ( u32 ) => {};
    ( $base: ty ) => {
        pub fn into_u16(self) -> u16 {
            self.0 .0 as u16
        }
    };
}

macro_rules! to_u8 {
    ( u64 ) => {};
    ( u32 ) => {};
    ( u16 ) => {};
    ( $base: ty ) => {
        pub fn into_u8(self) -> u8 {
            self.0 .0 as u8
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
    ( $base: ty ) => {
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

        impl $idx_name {
            pub fn is_valid(&self, c: $count_name) -> bool {
                self.0.is_valid(*c)
            }
        }
    };
}

macro_rules! specific {
    ( $base: ty, $idx_name:ident($inner_idx:ident), $count_name:ident ) => {
        // Declare our Index
        #[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, Default)]
        pub struct $idx_name(pub $inner_idx<$base>);

        impl $idx_name {
            to_u64!($base);
            to_u32!($base);
            to_u16!($base);
            to_u8!($base);
            to_usize!($base);
        }

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

        impl std::ops::Deref for $idx_name {
            type Target = $inner_idx<$base>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<$idx_name> for $base {
            fn from(i: $idx_name) -> $base {
                i.0 .0
            }
        }

        impl_add!($inner_idx, $base, $idx_name, $count_name);

        impl std::fmt::Display for $idx_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}({})", stringify!($idx_name), self.0)
            }
        }

        // Declare our Count
        #[derive(PartialEq, Eq, Debug, Copy, Clone)]
        pub struct $count_name(pub Count<$base>);

        impl $count_name {
            to_u64!($base);
            to_u32!($base);
            to_u16!($base);
            to_u8!($base);
            to_usize!($base);
        }

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

        impl std::ops::Deref for $count_name {
            type Target = Count<$base>;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<$count_name> for $base {
            fn from(c: $count_name) -> $base {
                c.0 .0
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
                write!(f, "{}({})", stringify!($count_name), self.0)
            }
        }
    };
}
