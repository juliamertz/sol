extern variadic func printf(fmt: Str) -> i32

struct Vec2 =
  x : f64
  y : f64
end

impl Vec2 =
  func add(self: Vec2, other: Vec2) -> Vec2
    Vec2 {
      x: self.x + other.x
      y: self.y + other.y
    }
  end

  func sub(self: Vec2, other: Vec2) -> Vec2
    Vec2 {
      x: self.x - other.x
      y: self.y - other.y
    }
  end
end

func main() -> i32
  let base = Vec2 { x: 10.0, y: 20.0 }
  let to_add = Vec2 { x: 5.0, y: 10.0 }
  let to_sub = Vec2 { x: 2.5, y: 5.0 }

  let result = base.add(to_add).sub(to_sub)
  printf("x: %f, y: %f", result.x, result.y)
end
