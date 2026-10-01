use super::*;

impl Parser<'_> {
    pub(super) fn parse_type(&mut self) -> PResult<AstNode<Type>> {
        let ident = self.expect_ident(KwSet::STRICT)?;
        let span = ident.span;
        Ok(AstNode::new(Type::Variable(*ident), span))
    }
}
