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

    fn format_float(f: f64) -> String {
        let s = f.to_string();

        if s.contains('.') {
            return s;
        }

        if let Some((mantissa, exponent)) = s.split_once(['e', 'E']) {
            return format!("{mantissa}.0e{exponent}");
        }

        format!("{s}.0")
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
            Stmt::Variable { pattern, kind, ty } => {
                let (kind, init) = match kind {
                    VariableKind::Let(init) => ("Let", init.as_ref()),
                    VariableKind::Const(init) => ("Const", Some(init)),
                };
                self.indentation += 1;
                write!(self.f, "{}(\n{}", kind, self.indent())?;
                self.pattern(pattern)?;
                if let Some(expr) = init {
                    write!(self.f, "\n{}", self.indent())?;
                    self.expr(expr)?;
                }
                if let Some(ty) = ty {
                    write!(self.f, "\n{}Type(", self.indent())?;
                    self.ty(ty)?;
                    write!(self.f, ")")?;
                }
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
            }
            Stmt::Assign { target, value } => {
                self.indentation += 1;
                write!(self.f, "Assign(\n{}", self.indent())?;
                self.assign_target(target)?;
                write!(self.f, "\n{}", self.indent())?;
                self.expr(value)?;
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
            }
        }
    }

    pub fn assign_target(&mut self, assign_target: &AssignTarget) -> fmt::Result {
        match assign_target {
            AssignTarget::Variable(sym) => {
                write!(self.f, "{}", self.session.interner.borrow().get(*sym))
            }
            AssignTarget::Wildcard => write!(self.f, "_"),
            AssignTarget::Member { object, key } => {
                self.indentation += 1;
                write!(self.f, "Member(\n{}", self.indent())?;
                self.expr(object)?;
                write!(
                    self.f,
                    "\n{}{}",
                    self.indent(),
                    self.session.interner.borrow().get(key.value)
                )?;
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
            }
            AssignTarget::Index { object, index } => {
                self.indentation += 1;
                write!(self.f, "Index(\n{}", self.indent())?;
                self.expr(object)?;
                write!(self.f, "\n{}", self.indent())?;
                self.expr(index)?;
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
            }
            AssignTarget::Error => write!(self.f, "Error"),
        }
    }

    pub fn expr(&mut self, expr: &Expr) -> fmt::Result {
        match expr {
            Expr::Group(node) => {
                write!(self.f, "Group(")?;
                self.expr(node)?;
                write!(self.f, ")")
            }
            Expr::Tuple(items) => {
                write!(self.f, "Tuple(")?;
                self.indentation += 1;
                for item in items {
                    write!(self.f, "\n{}", self.indent())?;
                    self.expr(item)?;
                }
                self.indentation -= 1;
                if !items.is_empty() {
                    write!(self.f, "\n{}", self.indent())?;
                }
                write!(self.f, ")")
            }
            Expr::Literal(lit) => self.literal(lit),
            Expr::Template { head, parts } => {
                self.indentation += 1;
                write!(self.f, "Template(\n{}", self.indent())?;
                write!(self.f, "{:?}", self.session.interner.borrow().get(*head))?;
                for part in parts {
                    write!(self.f, "\n{}Sub(", self.indent())?;
                    self.expr(&part.0.value)?;
                    write!(
                        self.f,
                        ")\n{}{:?}",
                        self.indent(),
                        self.session.interner.borrow().get(part.1)
                    )?;
                }
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
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
            Expr::Chain { base, links } => {
                self.indentation += 1;
                write!(self.f, "Chain(\n{}", self.indent())?;
                self.expr(base)?;
                for link in links {
                    write!(self.f, "\n{}", self.indent())?;
                    self.chain_link(link)?;
                }
                self.indentation -= 1;
                write!(self.f, "\n{})", self.indent())
            }
        }
    }

    pub fn literal(&mut self, lit: &Literal) -> fmt::Result {
        match lit {
            Literal::Int(value) => write!(self.f, "{value}"),
            Literal::Float(value) => write!(self.f, "{}", Self::format_float(*value)),
            Literal::String(sym) => {
                write!(self.f, "{:?}", self.session.interner.borrow().get(*sym))
            }
            Literal::Bool(value) => write!(self.f, "{value}"),
            Literal::Null => write!(self.f, "null"),
        }
    }

    pub fn chain_link(&mut self, link: &ChainLink) -> fmt::Result {
        match link {
            ChainLink::Member { key, optional } => write!(
                self.f,
                "Member{}({})",
                if *optional { "Optional" } else { "" },
                self.session.interner.borrow().get(key.value),
            ),
            ChainLink::Index { index, optional } => {
                write!(self.f, "Index{}(", if *optional { "Optional" } else { "" },)?;
                self.expr(index)?;
                write!(self.f, ")")
            }
            ChainLink::Call { args, optional } => {
                write!(self.f, "Call{}(", if *optional { "Optional" } else { "" },)?;
                self.indentation += 1;
                for arg in args {
                    write!(self.f, "\n{}", self.indent())?;
                    self.expr(arg)?;
                }
                self.indentation -= 1;
                if !args.is_empty() {
                    write!(self.f, "\n{}", self.indent())?;
                }
                write!(self.f, ")")
            }
            ChainLink::NotNull => write!(self.f, "NotNull"),
        }
    }

    pub fn pattern(&mut self, pattern: &Pattern) -> fmt::Result {
        match pattern {
            Pattern::Variable(sym) => {
                write!(self.f, "{}", self.session.interner.borrow().get(*sym))
            }
            Pattern::Wildcard => write!(self.f, "_"),
        }
    }

    pub fn ty(&mut self, ty: &Type) -> fmt::Result {
        match ty {
            Type::Variable(sym) => write!(self.f, "{}", self.session.interner.borrow().get(*sym)),
            Type::Infer => write!(self.f, "_"),
        }
    }
}
