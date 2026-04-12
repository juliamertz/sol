use extern stdio

extern variadic func printf(format: Str) -> i32

func main() -> i32
    let mut n = 0
    n = 10
    printf("Result is %d", n)
    0
end
