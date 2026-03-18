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
