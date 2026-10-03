mod expr;
mod pattern;
mod printer;
mod stmt;
mod ty;

pub use expr::*;
pub use pattern::*;
pub use stmt::*;
pub use ty::*;

use printer::Printer;
use std::{fmt, ops::Deref};
use thin_vec::ThinVec;

use crate::{assert_size, interner::Symbol, session::ParseSession, span::Span};

type Child<T> = Box<AstNode<T>>;

#[derive(Default)]
pub struct Program {
    pub statements: Vec<AstNode<Stmt>>,
}

impl Program {
    pub fn display(&self, session: &ParseSession) -> impl fmt::Display {
        fmt::from_fn(|f| Printer::new(f, session).program(self))
    }
}

pub struct AstNode<T> {
    pub value: T,
    pub span: Span,
}

impl<T> AstNode<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub fn with_value<U>(&self, value: U) -> AstNode<U> {
        AstNode::new(value, self.span)
    }
}

impl<T> Deref for AstNode<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
