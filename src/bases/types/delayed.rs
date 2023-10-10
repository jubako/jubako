use std::cell::Cell;
use std::fmt;
use std::sync::{atomic, Arc};

#[derive(Default, Debug, Clone)]
#[repr(transparent)]
pub struct Late<T: Copy>(Cell<Option<T>>);

impl<T: Copy> Late<T> {
    pub fn is_set(&self) -> bool {
        self.0.take().is_some()
    }

    pub fn set(&self, value: T) {
        debug_assert!(!self.is_set());
        self.0.set(Some(value));
    }

    pub fn get(&self) -> T {
        let opt = self.0.get();
        opt.unwrap()
    }
}

trait MyCell<T> {
    fn get(&self) -> T;
}

impl<T, U> MyCell<T> for Cell<U>
where
    U: Copy + Into<T>,
{
    fn get(&self) -> T {
        Cell::get(self).into()
    }
}

// S : data Source (what is the real value stored in the Vow)
// V: data View (how the data is viewed (get) by the bound)

pub trait SyncType {
    type SyncType: std::fmt::Debug + Default + Sync + Send;
    fn to_self(sync_val: &Self::SyncType) -> Self;
    fn set(sync_val: &Self::SyncType, value: Self);
    fn new(value: Self) -> Self::SyncType;
}

impl SyncType for u8 {
    type SyncType = atomic::AtomicU8;

    fn to_self(sync_val: &Self::SyncType) -> Self {
        sync_val.load(atomic::Ordering::Relaxed)
    }

    fn set(sync_val: &Self::SyncType, value: Self) {
        sync_val.store(value, atomic::Ordering::Relaxed)
    }

    fn new(value: Self) -> Self::SyncType {
        Self::SyncType::new(value)
    }
}

impl SyncType for u16 {
    type SyncType = atomic::AtomicU16;

    fn to_self(sync_val: &Self::SyncType) -> Self {
        sync_val.load(atomic::Ordering::Relaxed)
    }

    fn set(sync_val: &Self::SyncType, value: Self) {
        sync_val.store(value, atomic::Ordering::Relaxed)
    }

    fn new(value: Self) -> Self::SyncType {
        Self::SyncType::new(value)
    }
}

impl SyncType for u32 {
    type SyncType = atomic::AtomicU32;

    fn to_self(sync_val: &Self::SyncType) -> Self {
        sync_val.load(atomic::Ordering::Relaxed)
    }

    fn set(sync_val: &Self::SyncType, value: Self) {
        sync_val.store(value, atomic::Ordering::Relaxed)
    }

    fn new(value: Self) -> Self::SyncType {
        Self::SyncType::new(value)
    }
}

impl SyncType for u64 {
    type SyncType = atomic::AtomicU64;

    fn to_self(sync_val: &Self::SyncType) -> Self {
        sync_val.load(atomic::Ordering::Relaxed)
    }

    fn set(sync_val: &Self::SyncType, value: Self) {
        sync_val.store(value, atomic::Ordering::Relaxed)
    }

    fn new(value: Self) -> Self::SyncType {
        Self::SyncType::new(value)
    }
}

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct Vow<S: Copy + SyncType>(Arc<<S as SyncType>::SyncType>);

impl<S> Vow<S>
where
    S: Copy + SyncType + fmt::Debug + PartialEq + 'static,
{
    pub fn new(s: S) -> Self {
        Self(Arc::new(<S as SyncType>::new(s)))
    }

    pub fn fulfil(&self, value: S) {
        <S as SyncType>::set(&self.0, value)
    }

    pub fn get(&self) -> S {
        <S as SyncType>::to_self(&self.0)
    }

    pub fn bind(&self) -> Bound<S> {
        Bound(Arc::clone(&self.0))
    }
}

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct Bound<S>(Arc<<S as SyncType>::SyncType>)
where
    S: Copy + SyncType + PartialEq;

impl<S> Bound<S>
where
    S: Copy + SyncType + PartialEq,
{
    pub fn get(&self) -> S {
        <S as SyncType>::to_self(&self.0)
    }
}

impl<S> PartialEq for Bound<S>
where
    S: Copy + SyncType + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct Word<T: Copy>(Arc<dyn Fn() -> T + Sync + Send>);

impl<T> std::fmt::Debug for Word<T>
where
    T: Copy + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Word").field(&self.get()).finish()
    }
}

impl<T: Copy> Word<T> {
    pub fn get(&self) -> T {
        self.0()
    }
}

impl<T: Copy + Sync + Send + 'static> From<T> for Word<T> {
    fn from(v: T) -> Self {
        Self(Arc::new(move || v))
    }
}

impl<S, V> From<Bound<S>> for Word<V>
where
    S: Copy + Into<V> + SyncType + Send + PartialEq + 'static,
    V: Copy,
{
    fn from(b: Bound<S>) -> Self {
        Self(Arc::new(move || b.get().into()))
    }
}

impl<V> From<Box<dyn Fn() -> V + Sync + Send>> for Word<V>
where
    V: Copy,
{
    fn from(f: Box<dyn Fn() -> V + Sync + Send>) -> Self {
        Self(f.into())
    }
}

impl<V> From<fn() -> V> for Word<V>
where
    V: Copy + 'static,
{
    fn from(f: fn() -> V) -> Self {
        Self(Arc::new(f))
    }
}

impl<T: Copy + std::cmp::PartialEq> PartialEq for Word<T> {
    fn eq(&self, other: &Word<T>) -> bool {
        self.get() == other.get()
    }
}
impl<T: Copy + std::cmp::Eq> Eq for Word<T> {}
