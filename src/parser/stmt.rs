use super::*;

impl Parser<'_> {
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
            Some(self.parse_type()?)
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
            start.to(self.cursor.prev().span),
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
            return self.parse_function_decl(FunctionKind::Normal, self.cursor.prev().span);
        } else if self.eat(Ident(kw::Const)) {
            let first = self.cursor.prev();
            if self.eat(Ident(kw::Function)) {
                return self.parse_function_decl(FunctionKind::Const, first.span);
            }
        } else if self.eat(Ident(kw::Async)) {
            let first = self.cursor.prev();
            if self.eat(Ident(kw::Function)) {
                return self.parse_function_decl(FunctionKind::Async, first.span);
            }
        }
        self.unexpected(self.cursor.first().span)
    }

    fn parse_stmt(&mut self) -> PResult<AstNode<Stmt>> {
        if matches!(
            self.cursor.first().kind,
            Ident(kw::Async) | Ident(kw::Const) | Ident(kw::Function)
        ) {
            return self.parse_decl();
        }
        if self.eat(Ident(kw::Return)) {
            let start = self.cursor.prev().span;
            if self.eat(Semi) {
                return Ok(AstNode::new(
                    Stmt::Return(None),
                    start.to(self.cursor.prev().span),
                ));
            }
            let expr = self.parse_expr()?;
            self.expect(Semi)?;
            Ok(AstNode::new(
                Stmt::Return(Some(expr.into())),
                start.to(self.cursor.prev().span),
            ))
        } else {
            let expr = self.parse_expr()?;
            self.expect(Semi)?;
            let span = expr.span.to(self.cursor.prev().span);
            Ok(AstNode::new(Stmt::Expr(expr.into()), span))
        }
    }
}
