use super::EntryIdx;
use std::ops::{Add, Sub};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Range<T>
where
    T: Copy + Sub,
{
    begin: T,
    end: T,
}

impl<T> Range<T>
where
    T: Copy + Sub + PartialOrd,
{
    pub fn new(begin: T, end: T) -> Self {
        debug_assert!(end >= begin);
        Self { begin, end }
    }

    pub fn new_from_size<S>(begin: T, size: S) -> Self
    where
        T: Add<S, Output = T>,
    {
        Self {
            begin,
            end: begin + size,
        }
    }

    pub fn begin(&self) -> T {
        self.begin
    }

    pub fn end(&self) -> T {
        self.end
    }

    pub fn size(&self) -> <T as Sub>::Output {
        self.end - self.begin
    }
}

pub type EntryRange = Range<EntryIdx>;
