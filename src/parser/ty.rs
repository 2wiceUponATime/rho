use super::*;

impl Parser<'_> {
    pub(super) fn parse_ty(&mut self) -> PResult<AstNode<Type>> {
        if self.eat(Ident(kw::Underscore)) {
            Ok(self.with_prev_span(Type::Infer))
        } else {
            let sym = self.expect_ident(KwSet::STRICT)?.value;
            Ok(self.with_prev_span(Type::Variable(sym)))
        }
    }
}
