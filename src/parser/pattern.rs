use super::*;

impl Parser<'_> {
    pub(super) fn parse_pattern(&mut self) -> PResult<AstNode<Pattern>> {
        let ident = self.expect_ident(KwSet::STRICT.without(kw::Underscore))?;
        let span = ident.span;
        Ok(AstNode::new(
            match ident.value {
                kw::Underscore => Pattern::Wildcard,
                _ => Pattern::Variable(*ident),
            },
            span,
        ))
    }
}
