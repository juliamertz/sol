# List

## Source

```newlang
struct Test =
    buf : Int[],
end
```

## Expected (AST)

```ron
[
    Stmnt(StructDef((
        ident: "Test",
        fields: [
            ("buf", List((Int, None))),
        ],
    ))),
]
```