mod ast;
mod codegen;
mod lexer;
mod parser;

use codegen::{Compiler, Emitter};
use lexer::Lexer;
use miette::Result;
use parser::{Parser, Prec};

fn main() -> Result<()> {
    let content = std::fs::read_to_string("./tests/fibonacci").unwrap();

    // let mut lex = Lexer::new(&content);
    // while let Some(token) = lex.read_token() {
    //     dbg!(token);
    // }
    // std::process::exit(0);

    let mut parser = Parser::new(content);
    let nodes = match parser.parse() {
        Ok(nodes) => nodes,
        Err(err) => {
            dbg!(&parser.tokens);
            return Err(err);
        }
    };

    // dbg!(&nodes);

    let mut emitter = codegen::C::default();
    // let out = emitter.emit(vec![ast::Node::Stmnt(ast::Stmnt::Fn(ast::Fn {
    //     name: "main".into(),
    //     return_ty: "int".into(),
    //     body: ast::Block {
    //         nodes: vec![ast::Node::Stmnt(ast::Stmnt::Ret(ast::Ret { expr }))],
    //     },
    // }))]);

    let out = emitter.emit(&nodes);

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
