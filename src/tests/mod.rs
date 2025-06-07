use std::{path::Path, rc::Weak};

pub struct Spec {
    pub name: String,
    pub source: String,
    pub ast: String,
}

pub fn parse(source: impl AsRef<Path>) -> Vec<Spec> {
    let content = std::fs::read_to_string(source).unwrap();
    let lines = content
        .lines()
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    let mut specs = vec![];

    for line in lines {
       if line.starts_with(";;")  {
           let name = line.strip_prefix(";;").unwrap().trim();
           let mut source = String::new();
       }
    }

specs
}
