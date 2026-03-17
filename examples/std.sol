extern func write() -> i32

struct String =
  inner : u8[]
  len : u64
end

func main() -> i32
  let l : String = String { inner: [], len: 0 }

  write(1, "hello", 6)
end

