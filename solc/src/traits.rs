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

pub trait CollectVec<T> {
    fn collect_vec(self) -> Vec<T>;
}

impl<I, T> CollectVec<T> for I
where
    I: Iterator<Item = T>,
{
    fn collect_vec(self) -> Vec<T> {
        self.collect()
    }
}

pub trait TransposeVec<T, E> {
    fn transpose_vec(self) -> Result<Vec<T>, E>;
}

impl<I, T, E> TransposeVec<T, E> for I
where
    I: Iterator<Item = Result<T, E>>,
{
    fn transpose_vec(self) -> Result<Vec<T>, E> {
        self.collect()
    }
}
