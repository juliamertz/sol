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
                        test.run();
                    }
                }
            )*
        };
    }

    generate_tests![structures, binop, call_expr, if_expr];
}
