pub mod ast;
mod cursor;
mod lexer;
mod token_cursor;

use std::num::ParseIntError;

pub use lexer::*;

use crate::{
    interner::{KwSet, Symbol, kw},
    parser::{TokenKind::*, ast::*, token_cursor::TokenCursor},
    session::{Diagnostic, Level, ParseSession},
    span::Span,
};

#[derive(Clone, Copy)]
pub enum ExpectKind {
    TokenKind(TokenKind),
    Ident,
}

impl ExpectKind {
    pub fn describe(self, session: &ParseSession) -> String {
        match self {
            Self::TokenKind(kind) => kind.describe(session),
            Self::Ident => "identifier".into(),
        }
    }
}

pub enum ParseErrorKind {
    Unexpected {
        expected: Box<[ExpectKind]>,
        found: TokenKind,
    },
}

pub struct ParseError {
    kind: ParseErrorKind,
    span: Span,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn to_diag(&self, session: &ParseSession) -> Diagnostic {
        let message = match &self.kind {
            ParseErrorKind::Unexpected { expected, found } => format!(
                "Expected {}{} but found {}",
                if expected.len() > 1 { "one of: " } else { "" },
                expected
                    .iter()
                    .map(|k| k.describe(session))
                    .collect::<Vec<_>>()
                    .join(", "),
                found.describe(session)
            ),
        };
        Diagnostic::new(Level::Error, self.span, message)
    }
}

pub type PResult<T> = Result<T, ParseError>;

enum Assoc {
    Left,
    Right,
}

pub struct Parser<'psess> {
    session: &'psess ParseSession,
    cursor: TokenCursor,
    expected: Vec<ExpectKind>,
}

impl<'psess> Parser<'psess> {
    pub fn new(lexer: Lexer<'psess, '_>) -> Self {
        Self {
            session: lexer.session,
            cursor: lexer.into(),
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

    fn unexpected<T>(&self, span: Span) -> PResult<T> {
        Err(ParseError::new(
            ParseErrorKind::Unexpected {
                expected: self.expected.as_slice().into(),
                found: self.cursor.first().kind,
            },
            span,
        ))
    }

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program::default();
        while !self.cursor.is_eof() {
            match self.parse_statement() {
                Ok(stmt) => program.statements.push(stmt),
                Err(err) => {
                    self.recover_to(&[Semi]);
                    self.session
                        .diagnostics
                        .borrow_mut()
                        .push(err.to_diag(self.session));
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

    fn parse_statement(&mut self) -> PResult<AstNode<Statement>> {
        let start = self.cursor.first().span;
        let expr = self.parse_binary_expr(1)?;
        let span = start.to(self.cursor.prev().span);
        self.expect(Semi)?;
        Ok(AstNode::new(Statement::Expr(expr.into()), span))
    }

    fn binary_op(kind: TokenKind) -> Option<(BinaryOp, u8)> {
        Some(match kind {
            Plus => (BinaryOp::Add, 1),
            Minus => (BinaryOp::Sub, 1),
            Star => (BinaryOp::Mul, 2),
            Slash => (BinaryOp::Div, 2),
            StarStar => (BinaryOp::Pow, 3),
            _ => return None,
        })
    }

    fn assoc(prec: u8) -> Assoc {
        match prec {
            1 => Assoc::Left,
            2 => Assoc::Left,
            3 => Assoc::Right,
            _ => unimplemented!(),
        }
    }

    fn parse_primary_expr(&mut self) -> PResult<AstNode<Expr>> {
        if self.eat(IntLiteral) {
            let span = self.cursor.prev().span;
            let text = self.session.text(span);
            Ok(AstNode::new(
                Expr::IntLiteral(text.parse().unwrap_or_else(|e: ParseIntError| {
                    self.emit(Diagnostic::new(
                        Level::Error,
                        span,
                        format!("Failed to parse integer: {e}"),
                    ));
                    0
                })),
                span,
            ))
        } else if self.eat(FloatLiteral) {
            let span = self.cursor.prev().span;
            let text = self.session.text(span);
            Ok(AstNode::new(
                Expr::FloatLiteral(text.parse().unwrap()),
                span,
            ))
        } else if let Some(sym) = self.eat_ident(kw::STRICT) {
            Ok(AstNode::new(Expr::Variable(sym), self.cursor.prev().span))
        } else if self.eat(OpenParen) {
            let start = self.cursor.prev().span;
            let expr = self.parse_binary_expr(1)?;
            let end = self.expect(CloseParen)?.span;
            let span = start.to(end);
            Ok(AstNode::new(Expr::Group(expr.into()), span))
        } else {
            self.unexpected(self.cursor.first().span)
        }
    }

    fn parse_unary_expr(&mut self) -> PResult<AstNode<Expr>> {
        if self.eat(Minus) {
            let start = self.cursor.prev().span;
            let node = self.parse_unary_expr()?;
            let span = start.to(self.cursor.prev().span);
            Ok(AstNode::new(Expr::Unary(node.into(), UnaryOp::Minus), span))
        } else {
            self.parse_primary_expr()
        }
    }

    fn parse_binary_expr(&mut self, min_prec: u8) -> PResult<AstNode<Expr>> {
        let mut lhs = self.parse_unary_expr()?;
        while let Some((op, prec)) = Self::binary_op(self.cursor.first().kind) {
            if prec < min_prec {
                break;
            }
            self.bump();
            let next_min_prec = match Self::assoc(prec) {
                Assoc::Left => prec + 1,
                Assoc::Right => prec,
            };
            let rhs = self.parse_binary_expr(next_min_prec)?;
            let span = lhs.span.to(rhs.span);
            lhs = AstNode::new(
                Expr::Binary {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    op,
                },
                span,
            );
        }
        Ok(lhs)
    }
}
