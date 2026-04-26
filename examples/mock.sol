extern use stdio
extern variadic func printf(format: Str) -> i32

func main() -> i32
  let a = 0xFF_u32 + 20.0
  printf("val: %f", a)
end
