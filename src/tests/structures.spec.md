# Struct declaration

## Source

```newlang
struct Point =
    x : Int,
    y : Int,
end
```

## Expected (AST)

```ron
[
  Stmnt(Struct((
    ident: "Point",
    fields: [
      (
        ident: "x",
        ty: Int,
      ),
      (
        ident: "y",
        ty: Int,
      ),
    ],
  ))),
]
```
