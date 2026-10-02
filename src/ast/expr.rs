use super::*;

pub enum ChainLink {
    Member {
        key: AstNode<Symbol>,
        optional: bool,
    },
    Index {
        index: Child<Expr>,
        optional: bool,
    },
    Call {
        args: Vec<AstNode<Expr>>,
        optional: bool,
    },
    NotNull,
}

pub enum Expr {
    Group(Child<Self>),
    Tuple(Vec<AstNode<Self>>),
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(Symbol),
    Variable(Symbol),
    Binary {
        lhs: Child<Self>,
        rhs: Child<Self>,
        op: BinaryOp,
    },
    Unary(Child<Self>, UnaryOp),
    Chain {
        base: Child<Expr>,
        links: Vec<AstNode<ChainLink>>,
    },
}

impl Expr {
    pub fn display(&self, session: &ParseSession) -> impl fmt::Display {
        fmt::from_fn(|f| Printer::new(f, session).expr(self))
    }
}

impl AstNode<Expr> {
    pub fn chain(base: AstNode<Expr>, links: Vec<AstNode<ChainLink>>) -> Self {
        match links.last() {
            Some(last) => {
                let span = base.span.to(last.span);
                Self::new(
                    Expr::Chain {
                        base: Box::new(base),
                        links,
                    },
                    span,
                )
            }
            None => base,
        }
    }
}

#[derive(Debug)]
pub enum BinaryOp {
    /// `x ** y`
    Power,
    /// `x * x`
    Multiply,
    /// `x / y`
    Divide,
    /// `x % y`
    Remainder,
    /// `x + y`
    Add,
    /// `x - y`
    Subtract,
    /// `x << y`
    LeftShift,
    /// `x >> y`
    RightShift,
    /// `x & y`
    BitAnd,
    /// `x ^ y`
    BitXor,
    /// `x | y`
    BitOr,
    /// `x < y`
    Less,
    /// `x <= y`
    LessEq,
    /// `x > y`
    Greater,
    /// `x >= y`
    GreaterEq,
    /// `x is T`
    Is,
    /// `x == y`
    Equal,
    /// `x != y`
    NotEqual,
    /// `x && y`
    LogicAnd,
    /// `x || y`
    LogicOr,
    /// `x ?? y`
    NullishCoalesce,
}

#[derive(Debug)]
pub enum UnaryOp {
    /// `!x`
    LogicNot,
    /// `~x`
    BitNot,
    /// `-x`
    Negate,
    /// `await x`
    Await,
}

assert_size!(Expr, 32);
