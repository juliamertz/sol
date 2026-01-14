struct Point =
  x : u32
  y : u32
end

extern func printf() -> i32

func main() -> i32
  let point : Point = Point {
    x: 10
    y: 20
  }

  printf("x: %d, y: %d", point.x, point.y)
end

-- vim:ft=sol
