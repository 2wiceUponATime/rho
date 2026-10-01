use std::borrow::Cow;

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

#[derive(PartialEq, Eq)]
enum Assoc {
    Left,
    Right,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LiteralType {
    SingleQuote,
    DoubleQuote,
}

#[derive(Clone, Copy)]
struct LiteralContext {
    lit_type: LiteralType,
    pub start: u32,
}

impl LiteralContext {
    pub fn new(lit_type: LiteralType, start: u32) -> Self {
        Self { lit_type, start }
    }
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

    fn parse_escape(&self, s: &str, ctx: &LiteralContext) -> (char, usize) {
        match s.chars().next().unwrap() {
            '\\' => ('\\', 1),
            'n' => ('\n', 1),
            'r' => ('\r', 1),
            't' => ('\t', 1),
            '0' => ('\0', 1),
            // todo: hexadecimal escapes
            '\'' if ctx.lit_type == LiteralType::SingleQuote => ('\'', 1),
            '"' if ctx.lit_type == LiteralType::DoubleQuote => ('"', 1),
            c => {
                self.emit(Diagnostic::new(
                    Level::Warning,
                    self.span(ctx.start - 1, ctx.start + c.len_utf8() as u32),
                    format!("unknown escape: '\\{}'", c),
                ));
                (c, 1)
            }
        }
    }

    fn unescape(
        &self,
        raw: &str,
        first: usize,
        mut ctx: LiteralContext,
    ) -> String {
        let mut out = String::with_capacity(raw.len());
        let mut rest = raw;
        let mut next = Some(first);
        while let Some(i) = next {
            out.push_str(&rest[..i]);
            ctx.start += i as u32 + 1;
            let (ch, len) = self.parse_escape(&rest[i + 1..], &ctx);
            out.push(ch);
            rest = &rest[i + 1 + len..];
            next = memchr::memchr(b'\\', rest.as_bytes());
        }
        out.push_str(rest);
        out
    }

    fn cook<'a>(&self, raw: &'a str, ctx: LiteralContext) -> Cow<'a, str> {
        if let Some(first) = memchr::memchr(b'\\', raw.as_bytes()) {
            Cow::Owned(self.unescape(raw, first, ctx))
        } else {
            Cow::Borrowed(raw)
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
                        format!("failed to parse integer: {e}"),
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
        } else if self.eat(StringLiteral) {
            let span = self.cursor.prev().span;
            let text = self.session.text(span);
            let lit_type = match text.as_bytes()[0] {
                b'\'' => LiteralType::SingleQuote,
                b'"' => LiteralType::DoubleQuote,
                _ => unreachable!(),
            };
            let text = &text[1..text.len() - 1];
            let cooked = self.cook(text, LiteralContext::new(lit_type, span.start + 1));
            Ok(AstNode::new(
                Expr::StringLiteral(self.session.interner.borrow_mut().intern(&cooked)),
                span,
            ))
        } else if let Some(sym) = self.eat_ident(KwSet::STRICT) {
            Ok(AstNode::new(Expr::Variable(sym), self.cursor.prev().span))
        } else if self.eat(OpenParen) {
            let start = self.cursor.prev().span;
            let expr = self.parse_expr()?;
            let end = self.expect(CloseParen)?.span;
            let span = start.to(end);
            Ok(AstNode::new(Expr::Group(expr.into()), span))
        } else {
            self.unexpected(self.cursor.first().span)
        }
    }

    fn parse_postfix_expr(&mut self) -> PResult<AstNode<Expr>> {
        let mut result = self.parse_primary_expr()?;
        loop {
            result = match self.cursor.first().kind {
                Dot => self.parse_member(result, false)?,
                QuestionDot => match self.cursor.second().kind {
                    Ident(_) => self.parse_member(result, true)?,
                    OpenBracket => self.parse_index(result, true)?,
                    OpenParen => self.parse_call(result, true)?,
                    _ => {
                        self.bump();
                        self.expected.extend_from_slice(&[
                            ExpectKind::Ident,
                            ExpectKind::TokenKind(OpenBracket),
                            ExpectKind::TokenKind(OpenParen),
                        ]);
                        return self.unexpected(self.cursor.first().span);
                    }
                },
                OpenBracket => self.parse_index(result, false)?,
                OpenParen => self.parse_call(result, false)?,
                Exclam => {
                    let start = result.span;
                    AstNode::new(
                        Expr::Unary(result.into(), UnaryOp::NotNull),
                        start.to(self.bump().span),
                    )
                }
                _ => {
                    self.expected.extend_from_slice(&[
                        ExpectKind::TokenKind(Dot),
                        ExpectKind::TokenKind(QuestionDot),
                    ]);
                    break;
                }
            }
        }
        Ok(result)
    }

    fn parse_member(&mut self, object: AstNode<Expr>, optional: bool) -> PResult<AstNode<Expr>> {
        let start = object.span;
        self.bump();
        let end = self.cursor.first().span;
        Ok(AstNode::new(
            Expr::Member {
                object: object.into(),
                key: self.expect_ident(KwSet::EMPTY)?,
                optional,
            },
            start.to(end),
        ))
    }

    fn parse_index(&mut self, object: AstNode<Expr>, optional: bool) -> PResult<AstNode<Expr>> {
        let start = object.span;
        self.bump();
        if optional {
            self.bump();
        }
        let index = self.parse_expr()?;
        let end = self.expect(CloseBracket)?.span;
        Ok(AstNode::new(
            Expr::Index {
                object: object.into(),
                index: index.into(),
                optional,
            },
            start.to(end),
        ))
    }

    fn parse_call(&mut self, callee: AstNode<Expr>, optional: bool) -> PResult<AstNode<Expr>> {
        let start = callee.span;
        let mut args = vec![];
        self.bump();
        if optional {
            self.bump();
        }
        while !self.eat(CloseParen) {
            args.push(Box::new(self.parse_expr()?));
            if !self.eat(Comma) {
                self.expect(CloseParen)?;
                break;
            }
        }
        let end = self.cursor.prev().span;
        Ok(AstNode::new(
            Expr::Call {
                callee: callee.into(),
                args,
                optional,
            },
            start.to(end),
        ))
    }

    fn parse_unary_expr(&mut self) -> PResult<AstNode<Expr>> {
        if self.eat(Exclam) {
            self.parse_rest_of_unary(UnaryOp::LogicNot)
        } else if self.eat(Tilde) {
            self.parse_rest_of_unary(UnaryOp::BitNot)
        } else if self.eat(Minus) {
            self.parse_rest_of_unary(UnaryOp::Negate)
        } else if self.eat(Ident(kw::Await)) {
            self.parse_rest_of_unary(UnaryOp::Await)
        } else {
            self.parse_postfix_expr()
        }
    }

    fn parse_rest_of_unary(&mut self, op: UnaryOp) -> PResult<AstNode<Expr>> {
        let start = self.cursor.prev().span;
        let node = self.parse_unary_expr()?;
        let span = start.to(self.cursor.prev().span);
        Ok(AstNode::new(Expr::Unary(node.into(), op), span))
    }

    fn parse_binary_expr(&mut self, min_prec: u8) -> PResult<AstNode<Expr>> {
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
            if Self::assoc(prec) == Assoc::None {
                let next = self.cursor.first();
                if Self::binary_op(next.kind).is_some_and(|(_, next_prec)| next_prec == prec) {
                    self.emit(Diagnostic::new(
                        Level::Error,
                        next.span,
                        "non-associative operators cannot be chained; use parentheses".to_string(),
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

    pub(super) fn parse_expr(&mut self) -> PResult<AstNode<Expr>> {
        self.parse_binary_expr(0)
    }
}
