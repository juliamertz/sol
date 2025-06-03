mod ast;
mod codegen;
mod lexer;
mod parser;

const CONTENT: &str = r#"10 + 20"#;

use codegen::{Compiler, Emitter};
use miette::Result;
use parser::{Parser, Precedence};

fn main() -> Result<()> {
    let mut parser = Parser::new(CONTENT);
    let expr = parser.expr(Precedence::default())?;
    dbg!(&expr);

    let mut emitter = codegen::C::default();
    let out = emitter.emit(vec![ast::Node::Stmnt(ast::Stmnt::Fn(ast::Fn {
        name: "main".into(),
        return_ty: "int".into(),
        body: ast::Block {
            nodes: vec![ast::Node::Stmnt(ast::Stmnt::Ret(ast::Ret { expr }))],
        },
    }))]);

    println!("{out}");

    let bin_path = emitter.build_exe(
        &out,
        "test",
        codegen::CCOpts {
            cleanup: true,
            release: codegen::ReleaseType::Fast,
        },
    );
    dbg!(bin_path);

    Ok(())
}
