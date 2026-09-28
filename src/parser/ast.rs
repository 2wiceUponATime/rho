use std::{fmt, ops::Deref};

use crate::{interner::Symbol, session::ParseSession, span::Span};

type Child<T> = Box<AstNode<T>>;

#[derive(Default)]
pub struct Program {
    pub statements: Vec<AstNode<Statement>>,
}

struct Printer<'a, 'f> {
    f: &'a mut fmt::Formatter<'f>,
    session: &'a ParseSession,
    indentation: usize,
}

impl<'a, 'f> Printer<'a, 'f> {
    pub fn new(f: &'a mut fmt::Formatter<'f>, session: &'a ParseSession) -> Self {
        Self {
            f,
            session,
            indentation: 0,
        }
    }

    const fn indent(&self) -> impl fmt::Display + use<> {
        let width = self.indentation * 2;
        fmt::from_fn(move |f| write!(f, "{:width$}", "", width = width))
    }

    pub fn program(&mut self, program: &Program) -> fmt::Result {
        let last = program.statements.len().saturating_sub(1);
        for (i, stmt) in program.statements.iter().enumerate() {
            self.statement(stmt)?;
            if i != last {
                writeln!(self.f)?;
            }
        }
        Ok(())
    }

    pub fn statement(&mut self, stmt: &Statement) -> fmt::Result {
        match stmt {
            Statement::Expr(node) => {
                write!(self.f, "Expr(")?;
                self.expr(node)?;
                write!(self.f, ")")?;
            }
        }
        Ok(())
    }

    pub fn expr(&mut self, expr: &Expr) -> fmt::Result {
        match expr {
            Expr::Group(node) => {
                write!(self.f, "Group(")?;
                self.expr(node)?;
                write!(self.f, ")")?;
            }
            Expr::IntLiteral(value) => write!(self.f, "Literal({value})")?,
            Expr::FloatLiteral(value) => write!(self.f, "Literal({value})")?,
            Expr::Variable(sym) => write!(
                self.f,
                "Variable({})",
                self.session.interner.borrow().get(*sym)
            )?,
            Expr::Binary { lhs, rhs, op } => {
                self.indentation += 1;
                write!(self.f, "Binary{op:?}(\n{}", self.indent())?;
                self.expr(lhs)?;
                write!(self.f, "\n{}", self.indent())?;
                self.expr(rhs)?;
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())?;
            }
            Expr::Unary(node, op) => {
                write!(self.f, "Unary{op:?}(")?;
                self.expr(node)?;
                write!(self.f, ")")?;
            }
        }
        Ok(())
    }
}

impl Program {
    pub fn display(&self, session: &ParseSession) -> impl fmt::Display {
        fmt::from_fn(|f| Printer::new(f, session).program(self))
    }
}

pub enum Statement {
    Expr(Child<Expr>),
}

impl Statement {
    pub fn display(&self, session: &ParseSession) -> impl fmt::Display {
        fmt::from_fn(|f| Printer::new(f, session).statement(self))
    }
}

pub enum Expr {
    Group(Child<Self>),
    IntLiteral(i64),
    FloatLiteral(f64),
    Variable(Symbol),
    Binary {
        lhs: Child<Self>,
        rhs: Child<Self>,
        op: BinaryOp,
    },
    Unary(Child<Self>, UnaryOp),
}

impl Expr {
    pub fn display(&self, session: &ParseSession) -> impl fmt::Display {
        fmt::from_fn(|f| Printer::new(f, session).expr(self))
    }
}

#[derive(Debug)]
pub enum BinaryOp {
    /// `a + b`
    Add,
    /// `a - b`
    Sub,
    /// `a * b`
    Mul,
    /// `a / b`
    Div,
    /// `a ** b`
    Pow,
}

#[derive(Debug)]
pub enum UnaryOp {
    Plus,
    Minus,
}

pub struct AstNode<T> {
    pub node: T,
    pub span: Span,
}

impl<T> AstNode<T> {
    pub fn new(node: T, span: Span) -> Self {
        Self { node, span }
    }
}

impl<T> Deref for AstNode<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}
