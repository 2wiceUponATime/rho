use super::*;

impl Parser<'_> {
    fn expr_to_assign_target(&mut self, expr: AstNode<Expr>) -> AssignTarget {
        let diag = || {
            Diagnostic::new(
                Level::Error,
                self.file_id,
                expr.span,
                "Only member and index expressions are allowed in assignment".to_owned(),
            )
        };

        match expr.value {
            Expr::Variable(sym) => match sym {
                kw::Underscore => AssignTarget::Wildcard,
                _ => AssignTarget::Variable(sym),
            },
            Expr::Chain { base, mut links } => match links.pop().unwrap().value {
                ChainLink::Member {
                    key,
                    optional: false,
                } => AssignTarget::Member {
                    object: Box::new(AstNode::chain(*base, links)),
                    key,
                },
                ChainLink::Index {
                    index,
                    optional: false,
                } => AssignTarget::Index {
                    object: Box::new(AstNode::chain(*base, links)),
                    index,
                },
                _ => {
                    self.emit(diag());
                    AssignTarget::Error
                }
            },
            _ => {
                self.emit(diag());
                AssignTarget::Error
            }
        }
    }

    fn parse_function_decl(&mut self, kind: FunctionKind, start: Span) -> PResult<AstNode<Stmt>> {
        let name = self.expect_ident(KwSet::STRICT)?;
        self.expect(OpenParen)?;
        let mut params = vec![];
        while !self.eat(CloseParen) {
            params.push(self.parse_pattern()?);
            if !self.eat(Comma) {
                self.expect(CloseParen)?;
                break;
            }
        }
        let return_type = if self.eat(MinusGt) {
            Some(Box::new(self.parse_ty()?))
        } else {
            None
        };
        Ok(AstNode::new(
            Stmt::Function {
                name: *name,
                kind,
                params,
                return_type,
                body: self.parse_block()?,
            },
            self.span_from(start),
        ))
    }

    fn parse_let(&mut self) -> PResult<AstNode<Stmt>> {
        let start = self.prev_span();
        let pattern = self.parse_pattern()?;
        let mut init = None;
        let mut ty = None;
        if self.eat(Colon) {
            ty = Some(self.parse_ty()?);
        }
        if self.eat(Eq) {
            init = Some(Box::new(self.parse_expr()?));
        }
        self.expect(Semi)?;
        Ok(AstNode::new(
            Stmt::Variable {
                pattern,
                kind: VariableKind::Let(init),
                ty,
            },
            self.span_from(start),
        ))
    }

    fn parse_const(&mut self) -> PResult<AstNode<Stmt>> {
        let start = self.prev_span();
        let pattern = self.parse_pattern()?;
        let mut ty = None;
        if self.eat(Colon) {
            ty = Some(self.parse_ty()?);
        }
        self.expect(Eq)?;
        let init = Box::new(self.parse_expr()?);
        self.expect(Semi)?;
        Ok(AstNode::new(
            Stmt::Variable {
                pattern,
                kind: VariableKind::Const(init),
                ty,
            },
            self.span_from(start),
        ))
    }

    fn parse_block(&mut self) -> PResult<Vec<AstNode<Stmt>>> {
        self.expect(OpenBrace)?;
        let mut result = vec![];
        while !self.eat(CloseBrace) {
            result.push(self.parse_stmt()?);
        }
        Ok(result)
    }

    pub(super) fn parse_decl(&mut self) -> PResult<AstNode<Stmt>> {
        if self.eat(Ident(kw::Function)) {
            return self.parse_function_decl(FunctionKind::Normal, self.prev_span());
        } else if self.eat(Ident(kw::Const)) {
            let first = self.cursor.prev();
            if self.eat(Ident(kw::Function)) {
                return self.parse_function_decl(FunctionKind::Const, first.span);
            }
            return self.parse_const();
        } else if self.eat(Ident(kw::Async)) {
            let first = self.cursor.prev();
            if self.eat(Ident(kw::Function)) {
                return self.parse_function_decl(FunctionKind::Async, first.span);
            }
        } else if self.eat(Ident(kw::Let)) {
            return self.parse_let();
        }
        self.unexpected(self.cursor.first().span)
    }

    fn parse_stmt(&mut self) -> PResult<AstNode<Stmt>> {
        if matches!(
            self.cursor.first().kind,
            Ident(kw::Async | kw::Const | kw::Function | kw::Let)
        ) {
            return self.parse_decl();
        }
        if self.eat(Ident(kw::Return)) {
            let start = self.prev_span();
            if self.eat(Semi) {
                return Ok(AstNode::new(Stmt::Return(None), self.span_from(start)));
            }
            let expr = self.parse_expr()?;
            self.expect(Semi)?;
            Ok(AstNode::new(
                Stmt::Return(Some(Box::new(expr))),
                self.span_from(start),
            ))
        } else {
            let expr = self.parse_expr()?;
            if self.eat(Eq) {
                let start = expr.span;
                let target = self.expr_to_assign_target(expr);
                let value = Box::new(self.parse_expr()?);
                self.expect(Semi)?;
                return Ok(AstNode::new(
                    Stmt::Assign { target, value },
                    self.span_from(start),
                ));
            }
            self.expect(Semi)?;
            let span = expr.span.to(self.prev_span());
            Ok(AstNode::new(Stmt::Expr(Box::new(expr)), span))
        }
    }
}
