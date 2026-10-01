use super::*;

pub struct Printer<'a, 'f> {
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
            self.stmt(stmt)?;
            if i != last {
                writeln!(self.f)?;
            }
        }
        Ok(())
    }

    pub fn stmt(&mut self, stmt: &Stmt) -> fmt::Result {
        match stmt {
            Stmt::Expr(expr) => {
                write!(self.f, "Expr(")?;
                self.expr(expr)?;
                write!(self.f, ")")
            }
            Stmt::Return(None) => write!(self.f, "Return"),
            Stmt::Return(Some(expr)) => {
                write!(self.f, "Return(")?;
                self.expr(expr)?;
                write!(self.f, ")")
            }
            Stmt::Function {
                name,
                kind,
                params,
                return_type,
                body,
            } => {
                self.indentation += 1;
                write!(self.f, "Function{:?}(\n{}", kind, self.indent())?;
                write!(self.f, "{}", self.session.interner.borrow().get(*name))?;
                if !params.is_empty() {
                    write!(self.f, "\n{}Params(", self.indent())?;
                    self.indentation += 1;
                    for pattern in params {
                        write!(self.f, "\n{}", self.indent())?;
                        self.pattern(pattern)?;
                    }
                    self.indentation -= 1;
                    write!(self.f, "\n{})", self.indent())?;
                }
                if let Some(ty) = return_type {
                    write!(self.f, "\n{}Returns(", self.indent())?;
                    self.ty(ty)?;
                    write!(self.f, ")")?;
                }
                for stmt in body {
                    write!(self.f, "\n{}", self.indent())?;
                    self.stmt(stmt)?;
                }
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
            }
        }
    }

    pub fn expr(&mut self, expr: &Expr) -> fmt::Result {
        match expr {
            Expr::Group(node) => {
                write!(self.f, "Group(")?;
                self.expr(node)?;
                write!(self.f, ")")
            }
            Expr::IntLiteral(value) => write!(self.f, "Literal({value})"),
            Expr::FloatLiteral(value) => write!(self.f, "Literal({value})"),
            Expr::StringLiteral(sym) => {
                let interner = self.session.interner.borrow();
                let text = interner.get(*sym);
                write!(self.f, "{:?}", text)
            }
            Expr::Variable(sym) => write!(
                self.f,
                "Variable({})",
                self.session.interner.borrow().get(*sym)
            ),
            Expr::Binary { lhs, rhs, op } => {
                self.indentation += 1;
                write!(self.f, "Binary{op:?}(\n{}", self.indent())?;
                self.expr(lhs)?;
                write!(self.f, "\n{}", self.indent())?;
                self.expr(rhs)?;
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
            }
            Expr::Unary(node, op) => {
                write!(self.f, "Unary{op:?}(")?;
                self.expr(node)?;
                write!(self.f, ")")
            }
            Expr::Member {
                object: lhs,
                key,
                optional,
            } => {
                self.indentation += 1;
                write!(
                    self.f,
                    "Member{}(\n{}",
                    if *optional { "Optional" } else { "" },
                    self.indent()
                )?;
                self.expr(lhs)?;
                write!(
                    self.f,
                    "\n{}{}",
                    self.indent(),
                    self.session.interner.borrow().get(**key)
                )?;
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
            }
            Expr::Index {
                object: lhs,
                index: rhs,
                optional,
            } => {
                self.indentation += 1;
                write!(
                    self.f,
                    "Index{}(\n{}",
                    if *optional { "Optional" } else { "" },
                    self.indent()
                )?;
                self.expr(lhs)?;
                write!(self.f, "\n{}", self.indent())?;
                self.expr(rhs)?;
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
            }
            Expr::Call {
                callee,
                args,
                optional,
            } => {
                self.indentation += 1;
                write!(
                    self.f,
                    "Call{}(\n{}",
                    if *optional { "Optional" } else { "" },
                    self.indent()
                )?;
                self.expr(callee)?;
                for arg in args {
                    write!(self.f, "\n{}", self.indent())?;
                    self.expr(arg)?;
                }
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
            }
        }
    }

    pub fn pattern(&mut self, pattern: &Pattern) -> fmt::Result {
        match pattern {
            Pattern::Variable(sym) => {
                write!(self.f, "{}", self.session.interner.borrow().get(*sym))
            }
        }
    }

    pub fn ty(&mut self, ty: &Type) -> fmt::Result {
        match ty {
            Type::Variable(sym) => write!(self.f, "{}", self.session.interner.borrow().get(*sym)),
        }
    }
}
