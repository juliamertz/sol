extern crate test;

use std::assert_matches;

use crate::lexer::{
    Lexer,
    TokenKind::{self, *},
};

fn lex(source: &'static str) -> Vec<TokenKind> {
    let lexer = Lexer::new("inline".into(), source);
    lexer
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
        .into_iter()
        .map(|tok| tok.kind)
        .collect()
}

#[test]
fn math_expr() {
    let tokens = lex(r"10 / 2 * 50 - 5");
    assert_matches!(
        tokens.as_slice(),
        &[Num(_), Slash, Num(_), Asterisk, Num(_), Sub, Num(_), Eof]
    );
}

#[test]
fn literals() {
    let tokens = lex(r#"10 true false 
    "hello world""#);
    assert_matches!(
        tokens.as_slice(),
        &[Num(_), True, False, Newline, String, Eof]
    );
}

#[test]
fn keywords() {
    let tokens = lex(r"extern variadic func struct then end if else");
    assert_matches!(
        tokens.as_slice(),
        &[Extern, Variadic, Fn, Struct, Then, End, If, Else, Eof]
    );
}

#[test]
fn integers() {
    let tokens = lex(r"128 -64");
    assert_matches!(tokens.as_slice(), &[Num(_), Sub, Num(_), Eof]);
}
