# Function statement

## Source

```newlang
func main() -> Int
  return 0;
end
```

## Expected (AST)

```ron
[
    Stmnt(Fn((
    name: "main",
    is_extern: false,
    args: [],
    return_ty: Int,
    body: Some(
        Block(
            nodes: [Stmnt(Ret((val: IntLit(0))))],
        )),
    )))
]
```