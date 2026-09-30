use std::{env, process};

use rho::{
    parser::{Lexer, Parser},
    session::{Level, ParseSession},
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("Usage: cargo run [file]");
        return;
    };
    let mut session = ParseSession::new();
    let id = session.add_source_file(path.into()).unwrap();
    let lexer = Lexer::new(id, &session, &session.source(id).text);
    let mut parser = Parser::new(lexer);
    println!("{}", parser.parse_program().display(&session));
    for diag in session.diagnostics.borrow().iter() {
        eprintln!("{}", session.display_diag(diag))
    }
    if session
        .diagnostics
        .borrow()
        .max_level
        .is_some_and(|l| l >= Level::Error)
    {
        process::exit(1);
    }
}
