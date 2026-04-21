extern use stdio
extern variadic func printf(format: Str) -> i32

func main() -> i32
    let mut result = 0

    while result < 25 do
      result = result + 1
      if result != 10 then
        printf("result is %d\n", result)
      end
    end

    printf("final result is %d\n", result)

    0
end

