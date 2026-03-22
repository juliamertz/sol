use solc::lexer::TokenKind::*;
use solc_macros::lex;

#[test]
fn math_expr() {
    let tokens = lex! {
        10 - 20 * 50 / 20
    };

    assert!(tokens[0].kind().is_int());
    assert!(tokens[1].kind().is_sub());
    assert!(tokens[2].kind().is_int());
    assert!(tokens[3].kind().is_asterisk());
    assert!(tokens[4].kind().is_int());
    assert!(tokens[5].kind().is_slash());
    assert!(tokens[6].kind().is_int());
}

#[test]
fn keywords() {
    let tokens = lex! {
        let func return if then else end use extern struct
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(
        kinds,
        vec![Let, Fn, Ret, If, Then, Else, End, Use, Extern, Struct, Eof]
    );
}

#[test]
fn identifiers() {
    let tokens = lex! {
        foo bar_baz _leading x1 snake_case
    };

    for token in tokens.iter().filter(|t| t.kind().is_ident()) {
        assert!(!token.text.is_empty());
    }

    assert_eq!(tokens[0].text, "foo");
    assert_eq!(tokens[1].text, "bar_baz");
    assert_eq!(tokens[2].text, "_leading");
    assert_eq!(tokens[3].text, "x1");
    assert_eq!(tokens[4].text, "snake_case");
}

#[test]
fn string_literal() {
    let tokens = lex! {
        "hello world"
    };

    assert!(tokens[0].kind().is_string());
    assert_eq!(tokens[0].text, "hello world");
}

#[test]
fn assignment() {
    let tokens = lex! {
        let x = 42
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(kinds, vec![Let, Ident, Assign, Int, Eof]);
    assert_eq!(tokens[1].text, "x");
    assert_eq!(tokens[3].text, "42");
}

#[test]
fn equality() {
    let tokens = lex! {
        x == 10
    };

    assert!(tokens[0].kind().is_ident());
    assert!(tokens[1].kind().is_eq());
    assert!(tokens[2].kind().is_int());
}

#[test]
fn arrow() {
    let tokens = lex! {
        func main() -> i32
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(kinds, vec![Fn, Ident, LParen, RParen, Arrow, Ident, Eof]);
}

#[test]
fn brackets_and_delimiters() {
    let tokens = lex! {
        [1, 2, 3]
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(
        kinds,
        vec![LBracket, Int, Comma, Int, Comma, Int, RBracket, Eof]
    );
}

#[test]
fn boolean_operators() {
    let tokens = lex! {
        x and y or z
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(kinds, vec![Ident, And, Ident, Or, Ident, Eof]);
}

#[test]
fn prefix_operators() {
    let tokens = lex! {
        !x -y &z
    };

    assert!(tokens[0].kind().is_bang());
    assert!(tokens[1].kind().is_ident());
    assert!(tokens[2].kind().is_sub());
    assert!(tokens[3].kind().is_ident());
    assert!(tokens[4].kind().is_ampersand());
    assert!(tokens[5].kind().is_ident());
}

#[test]
fn comparison_operators() {
    let tokens = lex! {
        a < b > c
    };

    assert!(tokens[0].kind().is_ident());
    assert!(tokens[1].kind().is_l_angle());
    assert!(tokens[2].kind().is_ident());
    assert!(tokens[3].kind().is_r_angle());
    assert!(tokens[4].kind().is_ident());
}

#[test]
fn struct_definition() {
    let tokens = lex! {
        struct Point
            x: i32
            y: i32
        end
    };

    let kinds: Vec<_> = tokens
        .iter()
        .map(|t| *t.kind())
        .filter(|k| !k.is_newline())
        .collect();
    assert_eq!(
        kinds,
        vec![
            Struct, Ident, Ident, Colon, Ident, Ident, Colon, Ident, End, Eof
        ]
    );
}

#[test]
fn member_access() {
    let tokens = lex! {
        point.x
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(kinds, vec![Ident, Dot, Ident, Eof]);
}

#[test]
fn function_call_with_args() {
    let tokens = lex! {
        printf("hello %d", 42)
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(kinds, vec![Ident, LParen, String, Comma, Int, RParen, Eof]);
}

#[test]
fn multiline_preserves_newlines() {
    let tokens = lex! {
        let x = 1
        let y = 2
    };

    let newline_count = tokens.iter().filter(|t| t.kind().is_newline()).count();
    assert!(newline_count >= 1);
}

#[test]
fn extern_func_declaration() {
    let tokens = lex! {
        extern func puts(s: Str) -> i32
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(
        kinds,
        vec![
            Extern, Fn, Ident, LParen, Ident, Colon, Ident, RParen, Arrow, Ident, Eof
        ]
    );
}

#[test]
fn if_else_block() {
    let tokens = lex! {
        if x == 0 then
            return 1
        else
            return 0
        end
    };

    let kinds: Vec<_> = tokens
        .iter()
        .map(|t| *t.kind())
        .filter(|k| !k.is_newline())
        .collect();
    assert_eq!(
        kinds,
        vec![If, Ident, Eq, Int, Then, Ret, Int, Else, Ret, Int, End, Eof]
    );
}

#[test]
fn use_extern() {
    let tokens = lex! {
        use extern stdio
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(kinds, vec![Use, Extern, Ident, Eof]);
}

#[test]
fn semicolons() {
    let tokens = lex! {
        a; b; c
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(kinds, vec![Ident, Semicolon, Ident, Semicolon, Ident, Eof]);
}

#[test]
fn integer_text_values() {
    let tokens = lex! {
        0 1 42 999
    };

    let ints: Vec<_> = tokens
        .iter()
        .filter(|t| t.kind().is_int())
        .map(|t| &*t.text)
        .collect();
    assert_eq!(ints, vec!["0", "1", "42", "999"]);
}

#[test]
fn add_operator() {
    let tokens = lex! {
        1 + 2
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(kinds, vec![Int, Add, Int, Eof]);
}

#[test]
fn colon_in_params() {
    let tokens = lex! {
        n: i32
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(kinds, vec![Ident, Colon, Ident, Eof]);
}

#[test]
fn nested_function_calls() {
    let tokens = lex! {
        fib(n - 1) + fib(n - 2)
    };

    let kinds: Vec<_> = tokens.iter().map(|t| *t.kind()).collect();
    assert_eq!(
        kinds,
        vec![
            Ident, LParen, Ident, Sub, Int, RParen, Add, Ident, LParen, Ident, Sub, Int, RParen,
            Eof,
        ]
    );
}
