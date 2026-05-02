extern variadic func printf(format: Str) -> i32

func fib(n: i32) -> i32
    if n == 0 or n == 1 then
        n
    else
        fib(n - 1) + fib(n - 2)
    end
end

func fast_fib(n: i32) -> i32
  if n < 2 then
    n
  else
    let mut curr = 0
    let mut prev1 = 1
    let mut prev2 = 0

    let mut i = 2
    while i < n + 1 do
      curr = prev1 + prev2
      prev2 = prev1
      prev1 = curr
      i = i + 1
    end

    curr
  end
end

func main() -> i32
    let result = fib(30)
    printf("Result is %d", result)
    0
end
