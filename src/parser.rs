mod error;
mod expr;
mod pattern;
mod stmt;
mod token_cursor;
mod ty;

use error::*;
use std::num::ParseIntError;
use thin_vec::thin_vec;

use crate::{
    ast::*,
    interner::{KwSet, Symbol, kw},
    lexer::{TokenKind::*, *},
    parser::token_cursor::TokenCursor,
    session::{Diagnostic, FileId, Level, ParseSession},
    span::Span,
};

pub struct Parser<'psess> {
    session: &'psess ParseSession,
    file_id: FileId,
    cursor: TokenCursor,
    expected: Vec<ExpectKind>,
}

impl<'psess> Parser<'psess> {
    pub fn new(session: &'psess ParseSession, file_id: FileId, tokens: Vec<Token>) -> Self {
        Self {
            session,
            file_id,
            cursor: TokenCursor::new(tokens),
            expected: vec![],
        }
    }

    fn bump(&mut self) -> Token {
        self.expected.clear();
        self.cursor.bump()
    }

    fn emit(&self, diag: Diagnostic) {
        self.session.diagnostics.borrow_mut().push(diag);
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.cursor.first().kind == kind
    }

    fn eat(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.bump();
            true
        } else {
            self.expected.push(ExpectKind::TokenKind(kind));
            false
        }
    }

    fn eat_ident(&mut self, exclude: KwSet) -> Option<Symbol> {
        match self.cursor.first().kind {
            Ident(sym) if !exclude.contains(sym) => {
                self.bump();
                Some(sym)
            }
            _ => {
                self.expected.push(ExpectKind::Ident);
                None
            }
        }
    }

    fn expect(&mut self, kind: TokenKind) -> PResult<Token> {
        let token = self.cursor.first();
        if token.kind == kind {
            self.bump();
            Ok(token)
        } else {
            self.expected.push(ExpectKind::TokenKind(kind));
            self.unexpected(token.span)
        }
    }

    fn expect_ident(&mut self, exclude: KwSet) -> PResult<AstNode<Symbol>> {
        let token = self.cursor.first();
        match token.kind {
            Ident(sym) if !exclude.contains(sym) => {
                self.bump();
                Ok(AstNode::new(sym, token.span))
            }
            _ => {
                self.expected.push(ExpectKind::Ident);
                self.unexpected(token.span)
            }
        }
    }

    fn unexpected<T>(&self, span: Span) -> PResult<T> {
        Err(ParseError::new(
            ParseErrorKind::Unexpected {
                expected: self.expected.as_slice().into(),
                found: self.cursor.first().kind,
            },
            span,
        ))
    }

    fn prev_span(&self) -> Span {
        self.cursor.prev().span
    }

    fn span_from(&self, start: Span) -> Span {
        start.to(self.prev_span())
    }

    fn with_prev_span<T>(&self, value: T) -> AstNode<T> {
        AstNode::new(value, self.prev_span())
    }

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program::default();
        while !self.cursor.is_eof() {
            match self.parse_decl() {
                Ok(stmt) => program.statements.push(stmt),
                Err(err) => {
                    self.recover_to(&[Semi]);
                    self.session
                        .diagnostics
                        .borrow_mut()
                        .push(err.to_diag(self.session, self.file_id));
                }
            };
        }
        program
    }

    pub fn recover_to(&mut self, kinds: &[TokenKind]) {
        while !self.cursor.is_eof() {
            let kind = self.cursor.bump().kind;
            if kinds.contains(&kind) {
                break;
            }
        }
    }
}
