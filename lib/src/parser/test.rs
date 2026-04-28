use std::assert_matches;

use crate::ast::*;

macro_rules! parse_expr {
    ($($toks:tt)*) => {{
        let source = stringify!($($toks)*);
        let file_path = ::std::path::PathBuf::from("inline");
        let mut parser = $crate::parser::Parser::new(file_path, source).expect("to construct parser");
        parser.expr($crate::parser::prec::Prec::default()).expect("to parse module")
    }};
}

#[test]
fn deref_expr() {
    assert_matches!(parse_expr! { *my_ptr }, Expr::Deref(_))
}

#[test]
fn deref_assign() {
    let expr = parse_expr! { *my_ptr = 255 };
    assert_matches!(expr, Expr::Assign(_));

    let Expr::Assign(assign) = expr else {
        unreachable!()
    };

    assert_matches!(assign.lhs.as_ref(), Expr::Deref(_));
}
