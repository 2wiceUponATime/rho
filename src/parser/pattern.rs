use super::*;

impl Parser<'_> {
    pub(super) fn parse_pattern(&mut self) -> PResult<AstNode<Pattern>> {
        if self.eat(Ident(kw::Underscore)) {
            Ok(self.with_prev_span(Pattern::Wildcard))
        } else {
            Ok(AstNode::new(
                Pattern::Variable(self.expect_ident(KwSet::STRICT)?.value),
                self.prev_span(),
            ))
        }
    }
}
