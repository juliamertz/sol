struct Point =
  x : u32
  y : u32
end

extern use stdio
extern variadic func printf(format: Str) -> i32

func main() -> i32
  let point = Point { x: 10 y: 20 }

  -- FIXME: we shouldn't be able to mutate fields of a non-mut struct
  point.x = 40

  printf("x: %d, y: %d", point.x, point.y)
end

-- vim:ft=sol
