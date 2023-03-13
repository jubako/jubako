use std::cell::Cell;
use std::rc::Rc;

#[derive(Default, Debug, Clone)]
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
pub struct Vow<S: Copy>(Rc<Cell<S>>);

impl<S: Copy + 'static + std::fmt::Debug> Vow<S> {
    pub fn new(s: S) -> Self {
        Self(Rc::new(Cell::new(s)))
    }

    pub fn fulfil(&self, value: S) {
        //        println!("Fulfil vow with {value:?}");
        self.0.set(value);
    }

    pub fn get(&self) -> S {
        self.0.as_ref().get()
    }

    pub fn bind(&self) -> Bound<S> {
        Bound(Rc::clone(&self.0))
    }

    /*pub fn bind(&self) -> Bound<S>
    where
        S: Into<V>,
        V: Copy,
    {
        Bound(Rc::clone(&self.0) as Rc<dyn MyCell<V>>)
    }*/
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bound<S: Copy>(Rc<Cell<S>>);

impl<S> Bound<S>
where
    S: Copy,
{
    pub fn get(&self) -> S {
        self.0.as_ref().get()
    }
}

#[derive(Clone)]
pub struct Generator<S: Copy, V>(Rc<Cell<S>>, fn(V) -> V);

impl<S, V> From<(Bound<S>, fn(V) -> V)> for Generator<S, V>
where
    S: Copy,
{
    fn from(other: (Bound<S>, fn(V) -> V)) -> Self {
        let (bound, func) = other;
        Self(Rc::clone(&bound.0), func)
    }
}

impl<T, U> MyCell<T> for (Cell<U>, fn(U) -> U)
where
    U: Copy + Into<T>,
{
    fn get(&self) -> T {
        let (cell, func) = self;
        func(Cell::get(cell)).into()
    }
}

#[derive(Clone)]
pub struct DynBound<V: Copy>(Rc<dyn MyCell<V>>, fn(V) -> V);

impl<V: Copy> DynBound<V> {
    pub fn get(&self) -> V {
        self.1(self.0.as_ref().get())
    }
}

impl<V> std::fmt::Debug for DynBound<V>
where
    V: Copy + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("DynBound").field(&self.get()).finish()
    }
}

impl<V> PartialEq for DynBound<V>
where
    V: Copy + std::cmp::PartialEq,
{
    fn eq(&self, other: &DynBound<V>) -> bool {
        self.get() == other.get()
    }
}

impl<S, V> From<Bound<S>> for DynBound<V>
where
    S: Copy + Into<V> + 'static,
    V: Copy,
{
    fn from(other: Bound<S>) -> DynBound<V> {
        DynBound(other.0 as Rc<dyn MyCell<V>>, std::convert::identity)
    }
}

impl<S, V> From<Generator<S, V>> for DynBound<V>
where
    S: Copy + Into<V> + 'static,
    V: Copy,
{
    fn from(other: Generator<S, V>) -> DynBound<V> {
        DynBound(other.0 as Rc<dyn MyCell<V>>, other.1)
    }
}

#[derive(Debug, Clone)]
pub enum Word<T: Copy> {
    Now(T),
    Later(DynBound<T>),
}

impl<T: Copy> Word<T> {
    pub fn get(&self) -> T {
        match self {
            Self::Now(v) => *v,
            Self::Later(b) => b.get(),
        }
    }
}

impl<T: Copy> From<T> for Word<T> {
    fn from(v: T) -> Self {
        Self::Now(v)
    }
}

impl<S, V> From<Bound<S>> for Word<V>
where
    S: Copy + Into<V> + 'static,
    V: Copy,
{
    fn from(b: Bound<S>) -> Self {
        Self::Later(b.into())
    }
}

impl<S, V> From<Generator<S, V>> for Word<V>
where
    S: Copy + Into<V> + 'static,
    V: Copy,
{
    fn from(my_cell: Generator<S, V>) -> Self {
        Self::Later(my_cell.into())
    }
}

impl<T: Copy + std::cmp::PartialEq> PartialEq for Word<T> {
    fn eq(&self, other: &Word<T>) -> bool {
        self.get() == other.get()
    }
}
impl<T: Copy + std::cmp::Eq> Eq for Word<T> {}
