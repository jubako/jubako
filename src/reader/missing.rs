use crate::common::PackInfo;

#[derive(Debug)]
pub enum MayMissPack<T> {
    MISSING(PackInfo),
    FOUND(T),
}

impl<T> MayMissPack<T> {
    #[inline]
    pub fn map<U, F>(self, f: F) -> MayMissPack<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::FOUND(x) => MayMissPack::FOUND(f(x)),
            Self::MISSING(pack_info) => MayMissPack::MISSING(pack_info),
        }
    }

    #[inline]
    pub fn unwrap(self) -> T {
        if let Self::FOUND(x) = self {
            return x;
        }
        panic!("called `MayMissPack::unwrap()` on a `MISSING` value");
    }

    pub fn get(self) -> Option<T> {
        if let Self::FOUND(x) = self {
            Some(x)
        } else {
            None
        }
    }

    pub fn as_ref(&self) -> MayMissPack<&T> {
        match self {
            Self::FOUND(x) => MayMissPack::FOUND(x),
            Self::MISSING(pack_info) => MayMissPack::MISSING(pack_info.clone()),
        }
    }
}

impl<T, E> MayMissPack<Result<T, E>> {
    #[inline]
    pub fn transpose(self) -> Result<MayMissPack<T>, E> {
        match self {
            Self::FOUND(Ok(x)) => Ok(MayMissPack::FOUND(x)),
            Self::FOUND(Err(e)) => Err(e),
            Self::MISSING(pack_info) => Ok(MayMissPack::MISSING(pack_info)),
        }
    }
}
