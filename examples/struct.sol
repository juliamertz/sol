struct Vector2 =
  x : f64
  y : f64
end

impl Vector2 =
  func add(self: Vector2, other: Vector2) -> Vector2
    Vector2 {
      x: self.x + other.x
      y: self.y + other.y
    }
  end
end

extern use stdio
extern variadic func printf(format: Str) -> i32

func main() -> i32
  let point = Vector2 { x: 10.0, y: 20.0 }

  printf("x: %f, y: %f", point.x, point.y)
end

-- vim:ft=sol
