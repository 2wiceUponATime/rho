use super::*;

#[derive(Debug)]
pub enum FunctionKind {
    Normal,
    Const,
    Async,
}

pub enum VariableKind {
    Let(Option<Child<Expr>>),
    Const(Child<Expr>),
}

pub enum Stmt {
    Expr(Child<Expr>),
    Return(Option<Child<Expr>>),
    Function {
        name: Symbol,
        kind: FunctionKind,
        params: Vec<AstNode<Pattern>>,
        return_type: Option<Child<Type>>,
        body: Vec<AstNode<Self>>,
    },
    Variable {
        pattern: AstNode<Pattern>,
        kind: VariableKind,
        ty: Option<AstNode<Type>>,
    },
    Assign {
        target: AssignTarget,
        value: Child<Expr>,
    },
}

impl Stmt {
    pub fn display(&self, session: &ParseSession) -> impl fmt::Display {
        fmt::from_fn(|f| Printer::new(f, session).stmt(self))
    }
}

pub enum AssignTarget {
    Variable(Symbol),
    Wildcard,
    Member {
        object: Child<Expr>,
        key: AstNode<Symbol>,
    },
    Index {
        object: Child<Expr>,
        index: Child<Expr>,
    },
    Error,
}

assert_size!(Stmt, 64);
assert_size!(AssignTarget, 24);
