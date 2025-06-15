# Add

## Source

```newlang
10 + 40
```

## Expected (AST)

```ron
[
    Expr(Infix((
        lhs: IntLit(10),
        op: Add,
        rhs: IntLit(40),
    ))),
]
```

# Sub

## Source

```newlang
200 - 100
```

## Expected (AST)

```ron
[
    Expr(Infix((
        lhs: IntLit(200),
        op: Sub,
        rhs: IntLit(100),
    ))),
]
```

# Mul

## Source

```newlang
200 * 100
```

## Expected (AST)

```ron
[
    Expr(Infix((
        lhs: IntLit(200),
        op: Mul,
        rhs: IntLit(100),
    ))),
]
```

# Eq

## Source

```newlang
200 == 100
```

## Expected (AST)

```ron
[
    Expr(Infix((
        lhs: IntLit(200),
        op: Eq,
        rhs: IntLit(100),
    ))),
]
```

# And

## Source

```newlang
200 and 100
```

## Expected (AST)

```ron
[
    Expr(Infix((
        lhs: IntLit(200),
        op: And,
        rhs: IntLit(100),
    ))),
]
```