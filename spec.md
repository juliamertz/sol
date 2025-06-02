This language should feel similar to lua but with some tweaks that are inspired by languages like rust and go

i'm probably making stuff way too complex !

## Open questions

- Explicit mutability
- Semicolons
- Do we want a type system (YES)
- I think function paradigm makes sense for the frontend, most of the time you're just iterating over some items and displaying them, how would this fit in to a lua-like language? (i really hope pipes work out)

### Variables

We deviate from lua a bit here

#### Local variables

```lua
let my_variable = "Hello world";
```

#### Global variables

Having the option for global mutability is nice in a loosy goosy lang.

probably do something like `var` if mutable otherwise `const` like in go

### Pattern matching

I definitely want an extensive pattern matching system like rust, this will be quite complex though so maybe do this later on 

```lua
-- i'm not good with examples but i like this syntax
let privilege = match ctx.get_role() with
    | Admin -> 0
    | Office { is_intern } -> if is_intern then 2 else 1
    | User -> 2
;;
```

### Functions

Both function statements and expressions

```lua
fn hello(name: String) -> String
    return "Hello" .. name; -- or just return by ending with an expression (like rust)
end
```

```lua
-- this would work, but i'm not sure i like it. 
-- could use a semicolon to do oneliners but would be ugly
let hello = (fn(name: String) -> String
    
end)

let m = (fn(name: String) -> String;  end)
```

### Enums

Not sure what would an enum implementation would look like for a lua-like language

an idea (ocaml-ish syntax but tagged enums like rust):

```
enum State =
  | Idle
  | Running   
  | Exited(Status)
```

### Structs

Not sure if structs are logical to have in an interpreted language (maybe have them as an addition to an unstructured kv object? maybe not), but i like the way rust does OOP, that way we also don't have a need for classes


