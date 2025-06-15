# List expression

## Source

```newlang
[10, 20, 30, fourty, 50];
```

## Expected (AST)

```ron
[
    Expr(List((items: [
        IntLit(10),
        IntLit(20),
        IntLit(30),
        Ident("fourty"),
        IntLit(50)
    ])))
]
```