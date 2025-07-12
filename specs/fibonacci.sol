use stdio;

extern func printf(format: Str) -> Int;

func fib(n: Int) -> Int
    if n == 0 or n == 1 then
        return n;
    end

    return fib(n - 1) + fib(n - 2);
end

func main() -> Int
    let result = fib(30);

    printf("Result is %d", result);
end

-- vim:ft=newlang
