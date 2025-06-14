use super::md;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::str::FromStr;

#[derive(Debug)]
pub enum AssertionKind {
    Eq,
    Ne,
}

// struct RawTest<'a> {
//     source_code: &'a str,
//     expected: &'a str,
// }

#[derive(Debug)]
pub struct Test<T: PartialEq + Eq> {
    name: String,
    kind: AssertionKind,
    expected: T,
    actual: T,
}

impl<T: PartialEq + Eq + Debug> Test<T> {
    fn run(&self) {
        match self.kind {
            AssertionKind::Eq => assert_eq!(self.expected, self.actual),
            AssertionKind::Ne => assert_ne!(self.expected, self.actual),
        }
    }
}

/// File containing multiple tests asserting the spec of this language
#[derive(Debug)]
pub struct Spec<'a, T: PartialEq + Eq + Deserialize<'a> + Serialize> {
    name: String,
    source: &'a str,
    tests: Vec<Test<T>>,
}

pub trait IntoSpec<'a, T: PartialEq + Eq + Deserialize<'a> + Serialize> {
    fn into_spec(&self) -> Spec<'a, T>;
}

impl<'a> IntoSpec<'a, Vec<crate::ast::Node>> for &'a str {
    fn into_spec(&self) -> Spec<'a, Vec<crate::ast::Node>> {
        let mut nodes = md::parse(self).into_iter();

        let mut tests = vec![];
        while let Some(node) = nodes.next() {
            let md::Node::Title {
                level: 1,
                text: name,
            } = node
            else {
                panic!("expected first node to be spec name");
            };

            assert_eq!(
                nodes.next(),
                Some(md::Node::Title {
                    level: 2,
                    text: "Source"
                })
            );

            let Some(md::Node::CodeBlock {
                kind: Some("newlang"),
                content: source_code,
            }) = nodes.next()
            else {
                panic!("source codeblock missing");
            };

            // skip expected  title
            // TODO: check
            nodes.next();

            let Some(md::Node::CodeBlock {
                kind: Some("ron"),
                content: expected,
            }) = nodes.next()
            else {
                panic!("expected codeblock missing");
            };

            let expected = ron::from_str(expected).unwrap();
            let actual = {
               let mut parser = crate::parser::Parser::new(&source_code);
               vec![parser.node().unwrap()]
            };

            // TODO: add lifetimes to avoid clones
            tests.push(Test {
                name: name.to_string(),
                kind: AssertionKind::Eq,
                expected,
                actual,
            });
            // dbg!(nodes);
            // let parser = crate::parser::Parser::new(self);
            // let actual =
        }

        todo!()
    }
}

// impl<'a, T> Spec<'a, T>
// where
//     T: PartialEq + Eq + Deserialize<'a> + Serialize,
// {
//     fn from_source(name: impl ToString, source: &'a str) -> Self {
//         Self {
//             name: name.to_string(),
//             source,
//             tests: vec![],
//         }
//     }
// }
