use super::*;
use crate::interner::kw;

macro_rules! binary_ops {
    ($($level:literal : $assoc:ident => {
        $($token:ident $( ( $($arg:tt)+ ) )? => $op:expr),* $(,)?
    })*) => {
        fn binary_op(kind: TokenKind) -> Option<(BinaryOp, u8)> {
            use BinaryOp::*;
            match kind {
                $($($token $(($($arg)+))? => Some(($op, $level)),)*)*
                _ => None,
            }
        }

        fn assoc(prec: u8) -> Assoc {
            match prec {
                $($level => Assoc::$assoc,)*
                _ => unreachable!(),
            }
        }
    };
}

enum Assoc {
    Left,
    Right,
    None,
}

impl Parser<'_> {
    binary_ops! {
        0: Left => {
            QuestionQuestion => NullishCoalesce,
        }
        1: Left => {
            PipePipe => LogicOr,
        }
        2: Left => {
            AmpAmp => LogicAnd,
        }
        3: None => {
            EqEq => Equal,
            ExclamEq => NotEqual,
        }
        4: None => {
            Lt => Less,
            LtEq => LessEq,
            Gt => Greater,
            GtEq => GreaterEq,
            // todo: special handling for `is` rhs
            Ident(kw::Is) => Is,
        }
        5: Left => {
            Pipe => BitOr,
        }
        6: Left => {
            Caret => BitXor,
        }
        7: Left => {
            Amp => BitAnd,
        }
        8: Left => {
            LtLt => LeftShift,
            GtGt => RightShift,
        }
        9: Left => {
            Plus => Add,
            Minus => Subtract,
        }
        10: Left => {
            Star => Multiply,
            Slash => Divide,
            Percent => Remainder,
        }
        11: Right => {
            StarStar => Power,
        }
    }

    fn parse_primary_expr(&mut self) -> PResult<AstNode<Expr>> {
        if self.eat(IntLiteral) {
            let span = self.cursor.prev().span;
            let text = self.session.text(span);
            Ok(AstNode::new(
                Expr::IntLiteral(text.parse().unwrap_or_else(|e: ParseIntError| {
                    self.emit(Diagnostic::new(
                        Level::Error,
                        span,
                        format!("Failed to parse integer: {e}"),
                    ));
                    0
                })),
                span,
            ))
        } else if self.eat(FloatLiteral) {
            let span = self.cursor.prev().span;
            let text = self.session.text(span);
            Ok(AstNode::new(
                Expr::FloatLiteral(text.parse().unwrap()),
                span,
            ))
        } else if let Some(sym) = self.eat_ident(KwSet::STRICT) {
            Ok(AstNode::new(Expr::Variable(sym), self.cursor.prev().span))
        } else if self.eat(OpenParen) {
            let start = self.cursor.prev().span;
            let expr = self.parse_binary_expr(0)?;
            let end = self.expect(CloseParen)?.span;
            let span = start.to(end);
            Ok(AstNode::new(Expr::Group(expr.into()), span))
        } else {
            self.unexpected(self.cursor.first().span)
        }
    }

    fn parse_unary_expr(&mut self) -> PResult<AstNode<Expr>> {
        if self.eat(Minus) {
            let start = self.cursor.prev().span;
            let node = self.parse_unary_expr()?;
            let span = start.to(self.cursor.prev().span);
            Ok(AstNode::new(Expr::Unary(node.into(), UnaryOp::Minus), span))
        } else {
            self.parse_primary_expr()
        }
    }

    pub(super) fn parse_binary_expr(&mut self, min_prec: u8) -> PResult<AstNode<Expr>> {
        let mut lhs = self.parse_unary_expr()?;
        while let Some((op, prec)) = Self::binary_op(self.cursor.first().kind) {
            if prec < min_prec {
                break;
            }
            self.bump();
            let next_min_prec = match Self::assoc(prec) {
                Assoc::Left | Assoc::None => prec + 1,
                Assoc::Right => prec,
            };
            let rhs = self.parse_binary_expr(next_min_prec)?;
            if matches!(Self::assoc(prec), Assoc::None) {
                let next = self.cursor.first();
                if Self::binary_op(next.kind).is_some_and(|(_, next_prec)| next_prec == prec) {
                    self.emit(Diagnostic::new(
                        Level::Error,
                        next.span,
                        "Non-associative operators cannot be chained; use parentheses".to_string(),
                    ));
                }
            }
            let span = lhs.span.to(rhs.span);
            lhs = AstNode::new(
                Expr::Binary {
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                    op,
                },
                span,
            );
        }
        Ok(lhs)
    }
}
