# Struct declaration

## Source

```newlang
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

```newlang
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
        val: Some(StructConstructor((
            ident: "Point",
            fields: [
                ("x", IntLit(10)),
                ("y", IntLit(20)),
            ],
        ))),
    ))),
]
```
