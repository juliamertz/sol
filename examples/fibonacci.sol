use stdio

extern func printf(format: Str) -> i32

func fib(n: Int) -> i32
    if n == 0 or n == 1 then
        return n
    end

    return fib(n - 1) + fib(n - 2)
end

func main() -> i32
    let result = fib(30)

    printf("Result is %d", result)
end

-- vim:ft=sol
