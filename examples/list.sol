use extern stdio

extern variadic func printf(format: Str) -> i32

func main() -> i32
    let items = [10, 250, 450]

    printf("0: %d\n", items[0])
    printf("1: %d\n", items[1])
    printf("2: %d\n", items[2])
end
