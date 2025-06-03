mod ast;
mod codegen;
mod lexer;
mod parser;

const CONTENT: &str = r#"
    func main() -> int
        return 0
    end
"#;

use std::process::exit;

use codegen::{Compiler, Emitter};
use lexer::Lexer;
use miette::Result;
use parser::{Parser, Precedence};

fn main() -> Result<()> {
    // let mut lex = Lexer::new(CONTENT);
    // while let Some(token) = lex.read_token() {
    //     dbg!(token);
    // }
    // exit(0);

    let mut parser = Parser::new(CONTENT);
    let nodes = parser.parse()?;
    dbg!(&nodes);

    let mut emitter = codegen::C::default();
    // let out = emitter.emit(vec![ast::Node::Stmnt(ast::Stmnt::Fn(ast::Fn {
    //     name: "main".into(),
    //     return_ty: "int".into(),
    //     body: ast::Block {
    //         nodes: vec![ast::Node::Stmnt(ast::Stmnt::Ret(ast::Ret { expr }))],
    //     },
    // }))]);

    let out = emitter.emit(nodes);

    println!("{out}");

    let bin_path = emitter.build_exe(
        &out,
        "test",
        codegen::CCOpts {
            cleanup: false,
            release: codegen::ReleaseType::Fast,
        },
    );
    dbg!(bin_path);

    Ok(())
}
