mod ast;
mod lexer;
mod parser;
mod codegen;

const CONTENT: &str = r#"10 + 20"#;

use codegen::Emitter;
use miette::Result;
use parser::{Parser, Precedence};

fn main() -> Result<()> {
    let mut parser = Parser::new(CONTENT);
    let expr = parser.expr(Precedence::default())?;
    dbg!(&expr);

    let mut c_code = codegen::C::default();
    let out = c_code.emit(vec![expr]);

    dbg!(out);

    Ok(())
}
