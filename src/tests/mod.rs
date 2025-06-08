use miette::{IntoDiagnostic, Result};
use std::{path::Path, rc::Weak};

pub struct Spec {
    pub name: String,
    pub source: String,
    pub ast: String,
}

pub fn parse(source: impl AsRef<Path>) -> Result<Vec<Spec>> {
    let mut buff = std::fs::read_to_string(source).into_diagnostic()?;

    Ok(vec![])
}

// fn parse_spec(buff: &mut String) -> Result<Spec> {

// }
