struct Point =
  x : Int,
  y : Int,
end


func make_point(x: Int) -> Point
  return Point{
    x : x,
    y : 10,
  }
end

extern func printf() -> Int;


func main() -> Int
  let point = Point{
    x : 10,
    y : 20,
  };

  printf("x: %d", point.x);
end

-- vim:ft=newlang
