use extern stdio

extern variadic func printf(format: Str) -> i32

func main() -> i32
    let mut result = 0

    while result < 25 do
      result = result + 1
      printf("result is %d\n", result)
    end

    printf("final result is %d\n", result)

    0
end

