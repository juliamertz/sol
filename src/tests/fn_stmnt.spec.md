# Function statement

## Source

```sol
func main() -> Int
  return 0
end
```

## Expected (AST)

```ron
[
    Stmnt(Fn((
        is_extern: false,
        name: "main",
        params: [],
        return_ty: Int,
        body: Some((
            nodes: [
                Stmnt(Ret((
                    val: IntLit(0),
                ))),
            ],
        )),
    ))),
]
```
