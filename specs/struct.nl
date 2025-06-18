struct Point =
  x : Int,
  y : Int,
end

extern func printf() -> Int;

func main() -> Int
  let point = Point{
    x : 10,
    y : 20,
  };

  let x_ref = &x;

  printf("x: %d", point.x);
end

-- vim:ft=newlang
