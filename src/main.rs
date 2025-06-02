type Value = String;

#[derive(Debug)]
struct Stack<const N: usize> {
    data: [Value; N],
    len: usize,
}

impl<const N: usize> Default for Stack<N> {
    fn default() -> Self {
        Self {
            data: [Value::default(); N],
            len: 0,
        }
    }
}

// impl<T: Copy + Default, const N: usize> Stack<T, N> {


fn main() {
    let mut stack: Stack<10> = Stack::default();
    dbg!(stack);

    println!("Hello, world!");
}
