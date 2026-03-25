pub trait Boxed {
    fn boxed(self) -> Box<Self>;
}

impl<T> Boxed for T
where
    T: Into<Box<T>>,
{
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
}

pub trait AsStr {
    fn as_str(&self) -> &str;
}

impl AsStr for &str {
    fn as_str(&self) -> &str {
        self
    }
}
