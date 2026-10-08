use std::{env, path::PathBuf, process};

use rho::{
    lexer::{Lexer, delims::match_delims},
    parser::Parser,
    session::{FilePath, ParseSession, SourceFile},
};

enum Mode {
    Lex,
    Delims,
    Parse,
}

fn main() {
    let args: Vec<_> = env::args().collect();
    let mode = match args.get(1).map(|s| s.as_str()) {
        Some("lex") => Mode::Lex,
        Some("delims") => Mode::Delims,
        Some("parse") => Mode::Parse,
        _ => {
            eprintln!("Usage: rho <lex|delims|parse> [src]");
            process::exit(2);
        }
    };
    if args.len() < 2 {
        println!("usage: rho <lex|delims|parse> [src]");
        process::exit(2);
    }

    let mut session = ParseSession::new();
    let id = if args.len() >= 3 {
        session.add_source(SourceFile::new(
            FilePath::Virtual("<arg>".to_owned()),
            args[2].clone(),
        ))
    } else {
        session.add_source_file(PathBuf::from("input.rho")).unwrap()
    };
    let lexer = Lexer::new(id, &session, &session.source(id).text);
    let tokens: Vec<_> = lexer.into_iter().collect();
    match mode {
        Mode::Lex => println!("{:#?}", tokens),
        Mode::Delims => println!("{:#?}", match_delims(&mut session, id, &tokens)),
        Mode::Parse => println!(
            "{}",
            Parser::new(&session, id, tokens)
                .parse_program()
                .display(&session)
        ),
    }
    let mut diags = session.diagnostics.borrow_mut();
    diags.sort();
    for diag in diags.iter() {
        eprintln!("{}", session.display_diag(diag))
    }
}
