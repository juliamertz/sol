# If expression

## Source

```sol
if n < 0 then
  return 0
end
```

## Expected (AST)

```ron
[
    Expr(If((
        condition: BinOp((
            lhs: Ident("n"),
            op: Lt,
            rhs: IntLit(0),
        )),
        consequence: Block(
            nodes: [Stmnt(Ret((val: IntLit(0))))],
        ),
        alternative: None,
    )))
]
```

# If Else expression

## Source

```sol
if n < 0 then
  return 0
else
  return 1
end
```

## Expected (AST)

```ron
[
    Expr(If((
        condition: BinOp((
            lhs: Ident("n"),
            op: Lt,
            rhs: IntLit(0),
        )),
        consequence: Block(
            nodes: [Stmnt(Ret((val: IntLit(0))))],
        ),
        alternative: Some(Block(
            nodes: [Stmnt(Ret((val: IntLit(1))))],
        ))
    )))
]
```
