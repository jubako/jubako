/// MayRef is a mix between a Cow and AsRef.
/// It is a enum as Cow but with only one feature : AsRef/Deref.
/// In opposition to Cow which has a `to_owned` and so enforce a clone on T,
/// MayRef inforce nothing on the (potentially) owned type.
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
