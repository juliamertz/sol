extern variadic func printf(format: Str) -> i32
extern func malloc(size: u64) -> *u8

struct String =
  bytes : *u8
end

func main() -> i32
  let numbers = malloc(10)

  let mut idx = 0u64
  while idx < 10u64 do
    let ptr = numbers + idx * 8u64
    let value = idx * 32u64
    *ptr = value
    printf("idx: %u, val: %u\n", idx, *ptr)
    idx = idx + 1u64
  end

  0
end
