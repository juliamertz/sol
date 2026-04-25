use crate::lexer::{
    Lexer,
    TokenKind::{self, *},
};

type Token = crate::lexer::Token<'static>;

fn lex(source: &'static str) -> Vec<Token> {
    let mut lexer = Lexer::new("inline".into(), source);
    let mut buf = vec![];

    while let Some(token) = lexer.read_token() {
        match token {
            Ok(tok) if tok.kind == Eof => break,
            Ok(tok) => buf.push(tok),
            Err(err) => panic!("failed to read token: {err}"),
        }
    }

    buf
}

fn kinds(tokens: Vec<Token>) -> Vec<TokenKind> {
    tokens.into_iter().map(|tok| tok.kind).collect()
}

#[test]
fn math_expr() {
    let tokens = kinds(lex(r"10 / 2 * 50 - 5"));
    assert_eq!(tokens, vec![Int, Slash, Int, Asterisk, Int, Sub, Int]);
}

#[test]
fn literals() {
    let tokens = kinds(lex(r#"10 true false "hello world""#));
    assert_eq!(tokens, vec![Int, True, False, String]);
}
