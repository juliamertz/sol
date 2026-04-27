extern use stdio
extern variadic func printf(format: Str) -> i32

func main() -> i32
  let a = 10_f64 + 20.0_f64
  printf("val: %f", a)
end
