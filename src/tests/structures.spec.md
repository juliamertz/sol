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
  Stmnt(StructDef((
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

# Struct instantiation

## Source

```newlang
let point = Point{
    x : 10,
    y : 20
};
```

## Expected (AST)

```ron
```
