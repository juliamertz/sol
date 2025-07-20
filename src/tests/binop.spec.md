# Add

## Source

```sol
10 + 40
```

## Expected (AST)

```ron
[
    Expr(BinOp((
        lhs: IntLit(10),
        op: Add,
        rhs: IntLit(40),
    ))),
]
```

# Sub

## Source

```sol
200 - 100
```

## Expected (AST)

```ron
[
    Expr(BinOp((
        lhs: IntLit(200),
        op: Sub,
        rhs: IntLit(100),
    ))),
]
```

# Mul

## Source

```sol
200 * 100
```

## Expected (AST)

```ron
[
    Expr(BinOp((
        lhs: IntLit(200),
        op: Mul,
        rhs: IntLit(100),
    ))),
]
```

# Eq

## Source

```sol
200 == 100
```

## Expected (AST)

```ron
[
    Expr(BinOp((
        lhs: IntLit(200),
        op: Eq,
        rhs: IntLit(100),
    ))),
]
```

# And

## Source

```sol
200 and 100
```

## Expected (AST)

```ron
[
    Expr(BinOp((
        lhs: IntLit(200),
        op: And,
        rhs: IntLit(100),
    ))),
]
```

# Nested

## Source

```sol
n == 0 or n == 1
```

## Expected (AST)

```ron
[
    Expr(BinOp((
        lhs: BinOp((
            lhs: Ident("n"),
            op: Eq,
            rhs: IntLit(0),
        )),
        op: Or,
        rhs: BinOp((
            lhs: Ident("n"),
            op: Eq,
            rhs: IntLit(1),
        )),
    ))),
]
```
