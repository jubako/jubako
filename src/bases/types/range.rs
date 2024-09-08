use super::{ASize, EntryIdx, Offset, Size};
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

pub(crate) type Region = Range<Offset>;

impl Region {
    /// Relative cut.
    /// offset and end are relative to the current region
    #[inline]
    pub fn cut_rel(&self, offset: Offset, size: Size) -> Self {
        let begin = self.begin() + offset;
        let end = begin + size;
        debug_assert!(
            end <= self.end(),
            "end({end:?}) <= self.end({:?})",
            self.end()
        );
        Self::new(begin, end)
    }
    #[inline]
    pub(crate) fn cut_rel_asize(&self, offset: Offset, size: ASize) -> ARegion {
        let begin = self.begin() + offset;
        let end = begin + Size::from(size);
        debug_assert!(
            end <= self.end(),
            "end({end:?}) <= self.end({:?})",
            self.end()
        );
        ARegion::new(begin, end)
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct ARegion(Region);

impl ARegion {
    pub fn new(begin: Offset, end: Offset) -> Self {
        debug_assert!(end >= begin);
        debug_assert!((end - begin).into_u64() < usize::MAX as u64);
        Self(Region { begin, end })
    }

    pub fn begin(&self) -> Offset {
        self.0.begin
    }

    pub fn end(&self) -> Offset {
        self.0.end
    }

    pub fn size(&self) -> ASize {
        ASize::new(self.0.size().into_u64() as usize)
    }
}

impl From<ARegion> for Region {
    fn from(value: ARegion) -> Self {
        value.0
    }
}

impl TryFrom<Region> for ARegion {
    type Error = std::num::TryFromIntError;
    fn try_from(value: Region) -> Result<Self, Self::Error> {
        let _size: ASize = value.size().try_into()?;
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::{Range, Region};
    use crate::{Offset, Size};

    #[test]
    fn test_empty_range() {
        let range = Range::new(5, 5);
        assert_eq!(range.begin(), 5);
        assert_eq!(range.end(), 5);
        assert_eq!(range.size(), 0);
        let range = Range::new_from_size(5, 0);
        assert_eq!(range.begin(), 5);
        assert_eq!(range.end(), 5);
        assert_eq!(range.size(), 0);
    }

    #[test]
    fn test_range() {
        let range = Range::new(5, 10);
        assert_eq!(range.begin(), 5);
        assert_eq!(range.end(), 10);
        assert_eq!(range.size(), 5);
        let range = Range::new_from_size(5, 5);
        assert_eq!(range.begin(), 5);
        assert_eq!(range.end(), 10);
        assert_eq!(range.size(), 5);
    }

    #[test]
    fn test_empty_region() {
        let region = Region::new_from_size(Offset::new(5), Size::zero());
        assert_eq!(region.begin(), Offset::new(5));
        assert_eq!(region.end(), Offset::new(5));
        assert_eq!(region.size(), Size::zero());

        let sub_region = region.cut_rel(Offset::zero(), Size::zero());
        assert_eq!(sub_region.begin(), Offset::new(5));
        assert_eq!(sub_region.end(), Offset::new(5));
        assert_eq!(sub_region.size(), Size::zero());

        // Offset too big
        let result = std::panic::catch_unwind(|| region.cut_rel(Offset::new(4), Size::zero()));
        assert!(result.is_err());

        // Size too big
        let result = std::panic::catch_unwind(|| region.cut_rel(Offset::zero(), Size::new(1)));
        assert!(result.is_err());
    }

    #[test]
    fn test_region() {
        let region = Region::new_from_size(Offset::new(5), Size::new(5));
        assert_eq!(region.begin(), Offset::new(5));
        assert_eq!(region.end(), Offset::new(10));
        assert_eq!(region.size(), Size::new(5));

        // Cut with empty size
        let sub_region = region.cut_rel(Offset::zero(), Size::zero());
        assert_eq!(sub_region.begin(), Offset::new(5));
        assert_eq!(sub_region.end(), Offset::new(5));
        assert_eq!(sub_region.size(), Size::zero());

        // Cut with offset at end
        let sub_region = region.cut_rel(Offset::new(5), Size::zero());
        assert_eq!(sub_region.begin(), Offset::new(10));
        assert_eq!(sub_region.end(), Offset::new(10));
        assert_eq!(sub_region.size(), Size::zero());

        // Cut with small offset and sized end
        let sub_region = region.cut_rel(Offset::new(1), Size::from(3_u64));
        assert_eq!(sub_region.begin(), Offset::new(6));
        assert_eq!(sub_region.end(), Offset::new(9));
        assert_eq!(sub_region.size(), Size::new(3));

        // Offset too big
        let result = std::panic::catch_unwind(|| region.cut_rel(Offset::new(6), Size::zero()));
        assert!(result.is_err());

        // Size too big
        let result = std::panic::catch_unwind(|| region.cut_rel(Offset::new(3), Size::from(3_u64)));
        assert!(result.is_err());
    }
}
