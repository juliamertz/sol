use stdio;

extern fn printf;

-- fiiibooooo
func fib(n: int) -> int
    if n == 0 or n == 1 then
        return n;
    end;

    return fib(n - 1) + fib(n - 2);
end

func main() -> int
    printf("fibbobibbo: %d", fib(30));
    return 0;
end
