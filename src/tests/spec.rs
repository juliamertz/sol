use super::md;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Debug, path::Path};

#[derive(Debug)]
pub struct Test<'a, T: PartialEq + Eq> {
    pub name: Cow<'a, str>,
    pub expected: T,
    pub actual: T,
}

#[derive(Debug)]
pub struct Spec<'a, T: PartialEq + Eq + Deserialize<'a> + Serialize> {
    pub _name: String,
    pub _source: &'a str,
    pub tests: Vec<Test<'a, T>>,
}

pub trait IntoSpec<'a, T: PartialEq + Eq + Deserialize<'a> + Serialize> {
    fn into_spec(&self, source: impl AsRef<Path>) -> Spec<'a, T>;
}

impl<'a> IntoSpec<'a, Vec<crate::lexer::TokenKind>> for &'a str {
    fn into_spec(&self, _source: impl AsRef<Path>) -> Spec<'a, Vec<crate::lexer::TokenKind>> {
        todo!()
    }
}

impl<'a> IntoSpec<'a, Vec<crate::ast::Node>> for &'a str {
    fn into_spec(&self, source: impl AsRef<Path>) -> Spec<'a, Vec<crate::ast::Node>> {
        let mut document = md::parse(self);
        let mut nodes = document.nodes.iter_mut();
        let mut write_back = false;

        let mut tests = vec![];
        while let Some(node) = nodes.next() {
            dbg!(&nodes, nodes.len());

            let md::Node::Title {
                level: 1,
                text: name,
            } = node
            else {
                panic!("expected first node to be spec name");
            };

            assert_eq!(
                nodes.next(),
                Some(&mut md::Node::Title {
                    level: 2,
                    text: Cow::Borrowed("Source")
                })
            );

            let Some(md::Node::CodeBlock {
                kind: Some(Cow::Borrowed("newlang")),
                content: source_code,
            }) = nodes.next()
            else {
                panic!("source codeblock missing");
            };

            // skip expected  title
            // TODO: check
            nodes.next();

            let Some(md::Node::CodeBlock {
                kind: Some(Cow::Borrowed("ron")),
                content: expected,
            }) = nodes.next()
            else {
                panic!("expected codeblock missing");
            };

            let actual = {
                let mut parser = crate::parser::Parser::new(&source_code);
                vec![parser.node().unwrap()]
            };
            let expected = if expected.is_empty() {
                let ser =
                    ron::ser::to_string_pretty(&actual, ron::ser::PrettyConfig::default()).unwrap();
                *expected = Cow::Owned(ser);
                write_back = true;

                actual.clone()
            } else {
                ron::from_str(&expected).unwrap()
            };

            tests.push(Test {
                name: name.clone(),
                expected,
                actual,
            });
        }

        if write_back {
            let rendered = document.to_markdown();
            std::fs::write(source.as_ref(), rendered).unwrap();
        }

        Spec {
            _name: "TODO: spec title".into(),
            _source: self,
            tests,
        }
    }
}
