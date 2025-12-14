use std::cell::Cell;

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
