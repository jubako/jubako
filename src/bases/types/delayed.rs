use std::cell::Cell;

#[derive(Default, Debug, Clone)]
pub struct Delayed<T: Copy>(Cell<Option<T>>);

impl<T: Copy> Delayed<T> {
    pub fn is_set(&self) -> bool {
        self.0.take().is_some()
    }

    pub fn set(&self, value: T) {
        assert!(!self.is_set());
        self.0.set(Some(value));
    }

    pub fn get(&self) -> T {
        let opt = self.0.get();
        opt.unwrap()
    }
}
