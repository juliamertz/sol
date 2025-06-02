mod lexer;
mod vm;

const CONTENT: &str = r#"10 + 20"#;

fn main() {
    let mut lex = lexer::Lexer::new(CONTENT);
    while let Some(token) = lex.read_token() {
        dbg!(token);
    }
}
