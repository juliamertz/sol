use miette::{IntoDiagnostic, Result};
use std::{path::Path, rc::Weak};

#[derive(Debug)]
pub struct Spec {
    pub name: String,
    pub source: String,
    pub ast: String,
}

pub fn parse(source: impl AsRef<Path>) -> Result<Spec> {
    let content = std::fs::read_to_string(source).into_diagnostic()?;
    let mut lines = content.split("\n").filter(|line| !line.is_empty());

    dbg!(&lines);

    let name = lines
        .next()
        .unwrap()
        .strip_prefix(";; test")
        .expect("prefix")
        .trim();

    let _ = lines
        .next()
        .unwrap()
        .strip_prefix(";; input")
        .expect("input tag");

    let mut input = String::new();
    while let Some(line) = lines.next() {
       if line.starts_with(";; output") {
          break;
       }
       input.push_str(line);
    }

    let mut output = String::new();
    while let Some(line) = lines.next() {
        output.push_str(line);
    }

    Ok(Spec {
        name: name.to_string(),
        source: input,
        ast: output,
    })
}
