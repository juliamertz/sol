# Struct declaration

## Source

```sol
fibonacci(n - 10)
```

## Expected (AST)

```ron
[
    Expr(Call((
        func: Ident("fibonacci"),
        params: [BinOp ((
            lhs: Ident("n"),
            op: Sub,
            rhs: IntLit(10),
        ))],
    )))
]
```
