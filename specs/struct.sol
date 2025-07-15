struct Point =
  x : Int
  y : Int
end

-- extern func printf() -> Int

func main() -> Int
  let point : Point = Point{ x: 10, y: 20 }

  printf("x: %d", point.x)
end

-- vim:ft=sol
