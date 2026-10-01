use super::*;

#[derive(Debug)]
pub enum FunctionKind {
    Normal,
    Const,
    Async,
}

pub enum Stmt {
    Expr(Child<Expr>),
    Return(Option<Child<Expr>>),
    Function {
        name: Symbol,
        kind: FunctionKind,
        params: Vec<AstNode<Pattern>>,
        return_type: Option<AstNode<Type>>,
        body: Vec<AstNode<Self>>,
    },
}

impl Stmt {
    pub fn display(&self, session: &ParseSession) -> impl fmt::Display {
        fmt::from_fn(|f| Printer::new(f, session).stmt(self))
    }
}
