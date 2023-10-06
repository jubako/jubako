use std::cell::Cell;
use std::fmt;
use std::sync::{Arc, Mutex};

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

#[derive(Debug, Default)]
#[repr(transparent)]
pub struct Vow<S: Copy>(Arc<Mutex<S>>);

impl<S> Vow<S>
where
    S: Copy + fmt::Debug + PartialEq + 'static,
{
    pub fn new(s: S) -> Self {
        Self(Arc::new(Mutex::new(s)))
    }

    pub fn fulfil(&self, value: S) {
        *self.0.lock().unwrap() = value;
    }

    pub fn get(&self) -> S {
        *self.0.lock().unwrap()
    }

    pub fn bind(&self) -> Bound<S> {
        Bound(Arc::clone(&self.0))
    }
}

#[derive(Clone, Debug)]
#[repr(transparent)]
pub struct Bound<S>(Arc<Mutex<S>>)
where
    S: Copy + PartialEq;

impl<S> Bound<S>
where
    S: Copy + PartialEq,
{
    pub fn get(&self) -> S {
        *self.0.lock().unwrap()
    }
}

impl<S> PartialEq for Bound<S>
where
    S: Copy + PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

#[repr(transparent)]
pub struct Word<T: Copy>(Box<dyn Fn() -> T + Sync + Send>);

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
        Self(Box::new(move || v))
    }
}

impl<S, V> From<Bound<S>> for Word<V>
where
    S: Copy + Into<V> + Send + PartialEq + 'static,
    V: Copy,
{
    fn from(b: Bound<S>) -> Self {
        Self(Box::new(move || b.get().into()))
    }
}

impl<V> From<Box<dyn Fn() -> V + Sync + Send>> for Word<V>
where
    V: Copy,
{
    fn from(f: Box<dyn Fn() -> V + Sync + Send>) -> Self {
        Self(f)
    }
}

impl<V> From<fn() -> V> for Word<V>
where
    V: Copy + 'static,
{
    fn from(f: fn() -> V) -> Self {
        Self(Box::new(f))
    }
}

impl<T: Copy + std::cmp::PartialEq> PartialEq for Word<T> {
    fn eq(&self, other: &Word<T>) -> bool {
        self.get() == other.get()
    }
}
impl<T: Copy + std::cmp::Eq> Eq for Word<T> {}
