use std::env;

use rho::{parser::Lexer, session::ParseSession};

fn main() {
    let args: Vec<String> = env::args().collect();
    let Some(path) = args.get(1) else {
        eprintln!("Usage: cargo run [file]");
        return;
    };
    let mut session = ParseSession::new();
    let id = session.add_source_file(path.into()).unwrap();
    let lexer = Lexer::new(id, &session, &session.source(id).text);
    for token in lexer {
        eprintln!("{:?} @ {}", token.kind, session.display_span(token.span));
    }
    for diag in session.diagnostics.borrow().iter() {
        eprintln!("{}", session.display_diag(diag))
    }
}
