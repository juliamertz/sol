func do_thing() -> Int
  let a = 10
  return a
end

func main() -> Int
  let width = 128
  let height = 128
  let size = width * height - 4
  if size > 10 then 
    return do_thing()
  end

  return size
end
