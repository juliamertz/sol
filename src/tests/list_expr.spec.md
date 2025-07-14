# List expression

## Source

```sol
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

# Empty List expression

## Source

```sol
[];
```

## Expected (AST)

```ron
[
    Expr(List((items: [ ])))
]
```

# Push

## Source

```sol
[];;
list_push(items, 20);
```

## Expected (AST)

```ron
[
    Expr(List((
        items: [],
    ))),
]
```
