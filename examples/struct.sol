struct Vector2 =
  x : u32
  y : u32
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
  let point = Vector2 { x: 10, y: 20 }
  let other_point = Vector2 { x: 5, y: 10 }

  -- FIXME: we shouldn't be able to mutate fields of a non-mut struct
  point.x = 40

  let final = point.add(other_point)

  printf("x: %d, y: %d", point.x, point.y)
end

-- vim:ft=sol
