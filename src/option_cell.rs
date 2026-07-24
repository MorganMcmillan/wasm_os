pub struct OptionCell<T>(Option<T>);

impl<T> OptionCell<T> {
    pub const fn none() -> Self {
        Self(None)
    }

    pub fn new(value: T) -> Self {
        Self(Some(value))
    }
}

impl<T> std::ops::Deref for OptionCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0
            .as_ref()
            .expect("Expected OptionCell to be initialized by now.")
    }
}

impl<T> std::ops::DerefMut for OptionCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
            .as_mut()
            .expect("Expected OptionCell to be initialized by now.")
    }
}
