# Return statement

## Source

```sol
return 0;
```

## Expected (AST)

```ron
[
    Stmnt(Ret((
        val: IntLit(0),
    )))
]
```
