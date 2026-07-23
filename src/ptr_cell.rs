/// A simple wrapper type for representing pointers as references
pub struct PtrCell<T: Sized> {
    pub inner: *mut T,
}

impl<T> PtrCell<T> {
    pub fn new(inner: *mut T) -> Self {
        Self { inner }
    }

    pub fn get(&self) -> &T {
        unsafe { std::mem::transmute(self.inner) }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { std::mem::transmute(self.inner) }
    }
}
