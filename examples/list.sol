extern use stdio
extern variadic func printf(format: Str) -> i32

func main() -> i32
    let items = [10, 250, 450]

    printf("0: %d, ", items[0])
    printf("1: %d, ", items[1])
    printf("2: %d", items[2])
end
