pub mod md;
pub mod spec;

use crate::ast::Node;
use crate::parser::Parser;
use spec::{IntoSpec, Spec};

mod parser {
    use super::*;

    macro_rules! generate_test {
        ($name:ident) => {
            #[test]
            fn $name() {
                let raw_spec = include_str!(concat!("./", stringify!($name), ".spec.md"));
                let spec: Spec<Vec<crate::ast::Node>> = raw_spec.into_spec();
                for test in spec.tests {
                    test.run();
                }
            }
        };
    }

    generate_test!(structures);
}
