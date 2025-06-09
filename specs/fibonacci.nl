-- vim:ft=newlang

use stdio;

extern func printf() -> int;

func fib(n: int) -> int
    if n == 0 or n == 1 then
        return n;
    end;

    return fib(n - 1) + fib(n - 2);
end

func main() -> int
    let result: int = fib(30);
    return 0;
end
