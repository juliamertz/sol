pub mod md;
pub mod spec;

use crate::ast::Node;
use spec::{IntoSpec, Spec};

mod parser {
    use super::*;

    macro_rules! generate_tests {
        ($($name:ident $(,)?)*) => {
            $(
                #[test]
                fn $name() {
                    let raw_spec = include_str!(concat!("./", stringify!($name), ".spec.md"));
                    let spec: Spec<Vec<Node>> = raw_spec.into_spec();
                    for test in spec.tests {
                        assert_eq!(
                            test.expected,
                            test.actual,
                            "{} test failed",
                            test.name
                        )
                    }
                }
            )*
        };
    }

    generate_tests![
        structures,
        binop,
        call_expr,
        if_expr,
        list_expr,
        return_stmnt
        fn_stmnt,
    ];
}
