-- for now a `use` statement will puts everything into the current scope
use std

extern func malloc(size: u64) -> *u8

func main() -> i32
  let str = String { inner: malloc(100) }

  0
end
