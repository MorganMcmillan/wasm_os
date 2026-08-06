pub struct MutCell<T>(T);

impl<T> MutCell<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    #[allow(mutable_transmutes)]
    pub fn borrow_static(&self) -> &'static mut T {
        // SAFETY: it's not.
        unsafe { std::mem::transmute(&self.0) }
    }

    pub fn as_static_ref(&self) -> &'static Self {
        // SAFETY: it's slightly more safe
        unsafe { std::mem::transmute::<&MutCell<T>, &'static MutCell<T>>(self) }
    }
}

unsafe impl<T> Send for MutCell<T> {}
unsafe impl<T> Sync for MutCell<T> {}

impl<T> std::ops::Deref for MutCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
