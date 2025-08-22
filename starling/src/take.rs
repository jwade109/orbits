pub struct Take<T>(Option<T>);

impl<T> Take<T> {
    pub fn new(val: T) -> Self {
        Self(Some(val))
    }

    pub fn from_opt(val: Option<T>) -> Self {
        Self(val)
    }

    pub fn take(&mut self) -> Option<T> {
        self.0.take()
    }

    pub fn peek(&self) -> Option<&T> {
        self.0.as_ref()
    }
}
