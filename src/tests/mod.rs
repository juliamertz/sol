pub mod md;
pub mod spec2;

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

mod spec {
    use super::*;

    #[derive(Debug)]
    pub struct Raw {
        pub source_code: String,
        pub expected: String,
    }

    #[derive(Debug)]
    pub struct Spec<T> {
        pub source: T,
        pub expected: T,
    }

    pub trait IntoSpec<T>
    where
        T: PartialEq + Eq,
    {
        fn into_spec(&self) -> Spec<T>;
    }

    impl<T: PartialEq + Eq> Spec<T> {
        pub fn eq(&self) -> bool {
            self.source == self.expected
        }
    }

    impl FromStr for Raw {
        type Err = ();

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let mut lines = s.lines().filter(|line| !line.is_empty());

            let line = lines.next().unwrap();
            assert_eq!(line, "# Source");

            let line = lines.next().unwrap();
            assert!(line.starts_with("```"));
            let lang = line.strip_prefix("```").unwrap();
            assert_eq!(lang, "newlang");

            let mut source_code = String::new();
            let mut line = lines.next().unwrap();
            while !line.starts_with("```") {
                source_code.push_str(line);
                line = lines.next().unwrap();
            }

            let line = lines.next().unwrap();
            assert!(line.starts_with("# Expected"));

            let line = lines.next().unwrap();
            assert!(line.starts_with("```"));
            let lang = line.strip_prefix("```").unwrap();
            assert_eq!(lang, "ron");

            let mut expected = String::new();
            let mut line = lines.next().unwrap();
            while !line.starts_with("```") {
                expected.push_str(line);
                line = lines.next().unwrap();
            }

            Ok(Self {
                source_code,
                expected,
            })
        }
    }
}

mod parser {
    use super::*;
    use crate::ast::Node;
    use crate::parser::Parser;
    use spec::{IntoSpec, Raw, Spec};

    // impl IntoSpec<Vec<Node>> for Raw {
    //     fn into_spec(&self) -> Spec<Vec<Node>> {
    //         let mut parser = Parser::new(&self.source_code);
    //         let source = vec![parser.node().unwrap()];
    //         let expected = ron::from_str(&self.expected).unwrap();

    //         Spec { source, expected }
    //     }
    // }

    // macro_rules! generate_test {
    //     ($name:ident) => {
    //         #[test]
    //         fn $name() {
    //             let raw_spec =
    //                 Raw::from_str(include_str!(concat!("./", stringify!($name), ".spec.md")))
    //                     .unwrap();
    //             let spec: Spec<Vec<Node>> = raw_spec.into_spec();
    //             assert_eq!(spec.source, spec.expected);
    //         }
    //     };
    // }

    // generate_test!(structures);

    // TODO: move over to new testing system

    // TODO: support multiple tests per spec file

    #[derive(Debug, Serialize, Deserialize)]
    pub struct OldSpec {
        pub name: String,
        pub source: String,
        pub expected: Vec<crate::ast::Node>,
    }

    macro_rules! generate_tests {
        ($path:expr,[$($i:ident$(,)?)*]) => {
            lazy_static! {
                static ref PARSED: Vec<OldSpec> = ron::from_str(include_str!($path)).unwrap();
            }

            $(
                #[test]
                fn $i() {
                    let spec = PARSED.iter().find(|spec| spec.name == stringify!($i)).unwrap();
                    let mut parser = crate::parser::Parser::new(spec.source.to_owned());
                    let parsed = parser.parse().unwrap();

                    if parsed != spec.expected {
                        std::fs::write(
                            "./test-expected",
                            ron::ser::to_string_pretty(&parsed, ron::ser::PrettyConfig::default())
                                .unwrap(),
                        ).unwrap();
                    }

                    assert_eq!(parsed, spec.expected);
                }
            )*
        };
    }

    generate_tests!(
        "parser.ron",
        [
            infix_expr_mul,
            infix_expr_eq,
            infix_expr_and,
            call_expr,
            if_expr,
            if_else_expr,
            list_expr,
            return_stmnt
            fn_stmnt
            struct_declr
        ]
    );
}

#[test]
fn aap() {
    use crate::tests::spec2::IntoSpec;

    let text = include_str!("./structures.spec.md");
    let spec = text.into_spec();

    std::process::exit(0);
}
