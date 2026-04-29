extern use stdio
extern use stdlib

extern variadic func printf(format: Str) -> i32
extern func malloc(size: u64) -> *u8

struct String =
  bytes : *u8
end

func main() -> i32
  let ptr = malloc(10)
  let ptr_with_offset = ptr + 8u64
  *ptr = 255
  *ptr_with_offset = 22
  printf("ptr: %u\n", *ptr)
  printf("ptr_with_offset: %u\n", *ptr_with_offset)
  0
end
