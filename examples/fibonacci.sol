use extern stdio

extern func printf(format: Str) -> i32

func fib(n: i32) -> i32
    if n == 0 or n == 1 then 
        return n
    else
        return fib(n - 1) + fib(n - 2)
    end
end

func main() -> i32
    let result = fib(30)

    printf("Result is %d", result)
end
