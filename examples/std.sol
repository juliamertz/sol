extern func write() -> i32

struct String =
  inner : u8[]
  len : u64
end

impl String =
  func as_bytes(self: String) -> *u8
    self.inner
  end
end
