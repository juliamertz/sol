use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;

mod parser {
    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Spec {
        pub name: String,
        pub source: String,
        pub expected: Vec<crate::ast::Node>,
    }

    lazy_static! {
        static ref PARSED: Vec<Spec> = ron::from_str(include_str!("./parser_tests.ron")).unwrap();
    }

    macro_rules! generate_tests {
        ($($i:ident$(,)?)*) => {
            $(
                #[test]
                fn $i() {
                    let spec = PARSED.iter().find(|spec| spec.name == stringify!($i)).unwrap();

                    let mut parser = crate::parser::Parser::new(spec.source.to_owned());
                    let ast = parser.parse().unwrap();

                        assert_eq!(ast, spec.expected);
                }
            )*
        };
    }

    generate_tests![infix_expr_mul, infix_expr_eq, if_expr, return_stmnt];
}
