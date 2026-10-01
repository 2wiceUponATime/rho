use super::*;

impl Parser<'_> {
    pub(super) fn parse_pattern(&mut self) -> PResult<AstNode<Pattern>> {
        let ident = self.expect_ident(KwSet::STRICT)?;
        let span = ident.span;
        Ok(AstNode::new(Pattern::Variable(*ident), span))
    }
}
