# Struct declaration

## Source

```newlang
fibonacci(n - 10)
```

## Expected (AST)

```ron
[
    Expr(Call((
        func: Ident("fibonacci"),
        args: [Infix ((
            lhs: Ident("n"),
            op: Sub,
            rhs: IntLit(10),
        ))],
    )))
]
```