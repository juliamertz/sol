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
                fn $name() -> ::std::result::Result<(), Box<dyn ::std::error::Error>> {
                    let raw_spec = include_str!(concat!("./", stringify!($name), ".spec.md"));
                    let filepath = format!("./src/tests/{}.spec.md", stringify!($name));
                    let spec: Spec<Vec<Node>> = match raw_spec.into_spec(&filepath) {
                        Ok(spec) => spec,
                        Err(err) => {
                            println!("failed to parse {filepath}, error: {err:?}");
                            return Err(err);
                        }
                    };
                    for test in spec.tests {
                        assert_eq!(
                            test.expected,
                            test.actual,
                            "{} test failed",
                            test.name
                        )
                    }
                    Ok(())
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
