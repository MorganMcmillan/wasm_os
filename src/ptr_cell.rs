/// A simple wrapper type for representing pointers as references
pub struct PtrCell<T: Sized> {
    pub inner: *mut T,
}

impl<T> PtrCell<T> {
    pub fn new(inner: *mut T) -> Self {
        Self { inner }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.inner }
    }

    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner }
    }
}

unsafe impl<T> Send for PtrCell<T> {}
unsafe impl<T> Sync for PtrCell<T> {}
