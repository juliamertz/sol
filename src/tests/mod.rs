use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

mod parser {
    use super::*;

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Spec {
        pub name: String,
        pub source: String,
        pub expected: Vec<crate::ast::Node>,
    }

    macro_rules! generate_tests {
        ($path:expr,[$($i:ident$(,)?)*]) => {
            lazy_static! {
                static ref PARSED: Vec<Spec> = ron::from_str(include_str!($path)).unwrap();
            }

            $(
                #[test]
                fn $i() {
                    let spec = PARSED.iter().find(|spec| spec.name == stringify!($i)).unwrap();
                    let mut parser = crate::parser::Parser::new(spec.source.to_owned());
                    assert_eq!(parser.parse().unwrap(), spec.expected);
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
        ]
    );
}
