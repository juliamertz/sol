use super::md;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, fmt::Debug};

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
    fn into_spec(&self) -> Spec<'a, T>;
}

impl<'a> IntoSpec<'a, Vec<crate::lexer::TokenKind>> for &'a str {
    fn into_spec(&self) -> Spec<'a, Vec<crate::lexer::TokenKind>> {
        todo!()
    }
}

impl<'a> IntoSpec<'a, Vec<crate::ast::Node>> for &'a str {
    fn into_spec(&self) -> Spec<'a, Vec<crate::ast::Node>> {
        let document = md::parse(self);
        let mut nodes = document.nodes.into_iter();

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
                Some(md::Node::Title {
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
                content: mut expected,
            }) = nodes.next()
            else {
                panic!("expected codeblock missing");
            };

            let actual = {
                let mut parser = crate::parser::Parser::new(&source_code);
                vec![parser.node().unwrap()]
            };
            let expected = if expected.is_empty() {
                eprintln!("Expected is empty for {name}, filling in with actual");
                expected = Cow::Owned("".into());
                actual.clone()
            } else {
                ron::from_str(&expected).unwrap()
            };


            panic!("len: {}, {nodes:?}", nodes.len());

            tests.push(Test {
                name,
                expected,
                actual,
            });
        }

        Spec {
            _name: "TODO: spec title".into(),
            _source: self,
            tests,
        }
    }
}
