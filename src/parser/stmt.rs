use super::*;

impl Parser<'_> {
    pub(super) fn parse_stmt(&mut self) -> PResult<AstNode<Statement>> {
        let start = self.cursor.first().span;
        let expr = self.parse_binary_expr(0)?;
        let span = start.to(self.cursor.prev().span);
        self.expect(Semi)?;
        Ok(AstNode::new(Statement::Expr(expr.into()), span))
    }
}
