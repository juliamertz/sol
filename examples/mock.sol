extern use stdio
extern use stdlib

extern variadic func printf(format: Str) -> i32
extern variadic func malloc(size: u64) -> *u8

struct String =
  bytes : *u8
end

func main() -> i32
  let mut ptr = malloc(10)
  ptr = 255
  printf("ptr: %u", *ptr)
  0
end
