use std::str::FromStr;

#[derive(Debug)]
pub enum Ty {
    Int,
    Bool,
    // Str,
    // List(Box<Ty>)
}

impl FromStr for Ty {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Int" => Ok(Self::Int),
            "Bool" => Ok(Self::Bool),
            _ => Err("No such type".into()),
        }
    }
}
