# Struct declaration

## Source

```sol
struct Point =
    x : Int
    y : Int
end
```

## Expected (AST)

```ron
[
    Stmnt(StructDef((
        ident: "Point",
        fields: [
            ("x", Int),
            ("y", Int),
        ],
    ))),
]
```

# Struct instantiation

## Source

```sol
let point = Point{
    x : 10
    y : 20
};
```

## Expected (AST)

```ron
[
    Stmnt(Let((
        name: "point",
        ty: None,
        val: Ident("Point"),
    ))),
]
```
