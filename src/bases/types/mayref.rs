pub enum MayRef<'a, T>
where
    T: 'a,
{
    Borrowed(&'a T),
    Owned(T),
}

impl<T> AsRef<T> for MayRef<'_, T> {
    fn as_ref(&self) -> &T {
        match self {
            Self::Borrowed(o) => o,
            Self::Owned(o) => o,
        }
    }
}

impl<T> std::ops::Deref for MayRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.as_ref()
    }
}
