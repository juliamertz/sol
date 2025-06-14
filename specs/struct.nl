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

  printf("hello world");
end

-- vim:ft=newlang
