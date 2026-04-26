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
  let i = Vector2 { x: 10, y: 20 }
  let j = Vector2 { x: 20, y: 30 }
  let k = i.add(j)

  printf("x: %d, y: %d", k.x, k.y)
end
