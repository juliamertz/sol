use super::md;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug)]
pub enum AssertionKind {
    Eq,
    Ne,
}

#[derive(Debug)]
pub struct Test<'a, T: PartialEq + Eq> {
    name: &'a str,
    kind: AssertionKind,
    expected: T,
    actual: T,
}

impl<T: PartialEq + Eq + Debug> Test<'_, T> {
    pub fn run(&self) {
        match self.kind {
            AssertionKind::Eq => assert_eq!(self.expected, self.actual),
            AssertionKind::Ne => assert_ne!(self.expected, self.actual),
        }
    }
}

/// File containing multiple tests asserting the spec of this language
#[derive(Debug)]
pub struct Spec<'a, T: PartialEq + Eq + Deserialize<'a> + Serialize> {
    pub name: String,
    pub source: &'a str,
    pub tests: Vec<Test<'a, T>>,
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

            tests.push(Test {
                name,
                kind: AssertionKind::Eq,
                expected,
                actual,
            });
        }

        Spec {
            name: "TODO: spec title".into(),
            source: self,
            tests,
        }
    }
}
