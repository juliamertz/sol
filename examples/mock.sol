use extern stdio

extern variadic func printf(format: Str) -> i32

func main() -> i32
    let mut l = [0, 0] 
    l[0] = 4294967295

    printf("Result is %u", l[0])
    return 0
end
