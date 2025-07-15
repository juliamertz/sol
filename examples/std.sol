extern func write() -> Int

struct String =
  inner : Int[]
  len : Int
end

func main() -> Int
  let l : String = String { inner: [], len: 0 }

  write(1, "hello", 6)
end

