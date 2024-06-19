use super::{End, EntryIdx, Offset, Size};
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

pub type Region = Range<Offset>;

impl Region {
    #[inline]
    pub fn new_to_end(begin: Offset, end: End, size: Size) -> Self {
        let end = match end {
            End::None => size.into(),
            End::Offset(o) => o,
            End::Size(s) => s.into(),
        };
        Self::new(begin, end)
    }

    /// Relative cut.
    /// offset and end are relative to the current region
    #[inline]
    pub fn cut_rel(&self, offset: Offset, end: End) -> Self {
        let begin = self.begin() + offset;
        let end = match end {
            End::None => self.end(),
            End::Offset(o) => self.begin() + o,
            End::Size(s) => begin + s,
        };
        debug_assert!(
            end <= self.end(),
            "end({end:?}) <= self.end({:?})",
            self.end()
        );
        Self::new(begin, end)
    }
}

#[cfg(test)]
mod tests {
    use super::{Range, Region};
    use crate::{End, Offset, Size};

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

        let sub_region = region.cut_rel(Offset::zero(), End::None);
        assert_eq!(sub_region.begin(), Offset::new(5));
        assert_eq!(sub_region.end(), Offset::new(5));
        assert_eq!(sub_region.size(), Size::zero());
        let sub_region = region.cut_rel(Offset::zero(), End::Offset(Offset::zero()));
        assert_eq!(sub_region.begin(), Offset::new(5));
        assert_eq!(sub_region.end(), Offset::new(5));
        assert_eq!(sub_region.size(), Size::zero());
        let sub_region = region.cut_rel(Offset::zero(), End::Size(Size::zero()));
        assert_eq!(sub_region.begin(), Offset::new(5));
        assert_eq!(sub_region.end(), Offset::new(5));
        assert_eq!(sub_region.size(), Size::zero());

        // Offset too big
        let result = std::panic::catch_unwind(|| region.cut_rel(Offset::new(4), End::None));
        assert!(result.is_err());

        // End Offset after end
        let result =
            std::panic::catch_unwind(|| region.cut_rel(Offset::zero(), End::new_offset(1_u64)));
        assert!(result.is_err());

        // Size too big
        let result =
            std::panic::catch_unwind(|| region.cut_rel(Offset::zero(), End::new_size(1_u64)));
        assert!(result.is_err());
    }

    #[test]
    fn test_region() {
        let region = Region::new_from_size(Offset::new(5), Size::new(5));
        assert_eq!(region.begin(), Offset::new(5));
        assert_eq!(region.end(), Offset::new(10));
        assert_eq!(region.size(), Size::new(5));

        // Cut with empty size
        let sub_region = region.cut_rel(Offset::zero(), End::Size(Size::zero()));
        assert_eq!(sub_region.begin(), Offset::new(5));
        assert_eq!(sub_region.end(), Offset::new(5));
        assert_eq!(sub_region.size(), Size::zero());

        // Cut with offset at end
        let sub_region = region.cut_rel(Offset::new(5), End::None);
        assert_eq!(sub_region.begin(), Offset::new(10));
        assert_eq!(sub_region.end(), Offset::new(10));
        assert_eq!(sub_region.size(), Size::zero());

        // Cut with small offset no end
        let sub_region = region.cut_rel(Offset::new(1), End::None);
        assert_eq!(sub_region.begin(), Offset::new(6));
        assert_eq!(sub_region.end(), Offset::new(10));
        assert_eq!(sub_region.size(), Size::new(4));

        // Cut with small offset and offset end
        let sub_region = region.cut_rel(Offset::new(1), End::new_offset(3_u64));
        assert_eq!(sub_region.begin(), Offset::new(6));
        assert_eq!(sub_region.end(), Offset::new(8));
        assert_eq!(sub_region.size(), Size::new(2));

        // Cut with small offset and sized end
        let sub_region = region.cut_rel(Offset::new(1), End::new_size(3_u64));
        assert_eq!(sub_region.begin(), Offset::new(6));
        assert_eq!(sub_region.end(), Offset::new(9));
        assert_eq!(sub_region.size(), Size::new(3));

        // Offset too big
        let result = std::panic::catch_unwind(|| region.cut_rel(Offset::new(6), End::None));
        assert!(result.is_err());

        // End Offset after end
        let result =
            std::panic::catch_unwind(|| region.cut_rel(Offset::new(1), End::new_offset(6_u64)));
        assert!(result.is_err());

        // Size too big
        let result =
            std::panic::catch_unwind(|| region.cut_rel(Offset::new(3), End::new_size(3_u64)));
        assert!(result.is_err());
    }
}
