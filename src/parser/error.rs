use super::*;

#[derive(Clone, Copy)]
pub enum ExpectKind {
    TokenKind(TokenKind),
    Ident,
}

impl ExpectKind {
    pub fn describe(self, session: &ParseSession) -> String {
        match self {
            Self::TokenKind(kind) => kind.describe(session),
            Self::Ident => "identifier".into(),
        }
    }
}

pub enum ParseErrorKind {
    Unexpected {
        expected: Box<[ExpectKind]>,
        found: TokenKind,
    },
}

pub struct ParseError {
    kind: ParseErrorKind,
    span: Span,
}

impl ParseError {
    pub fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }

    pub fn to_diag(&self, session: &ParseSession) -> Diagnostic {
        let message = match &self.kind {
            ParseErrorKind::Unexpected { expected, found } => format!(
                "Expected {}{} but found {}",
                if expected.len() > 1 { "one of: " } else { "" },
                expected
                    .iter()
                    .map(|k| k.describe(session))
                    .collect::<Vec<_>>()
                    .join(", "),
                found.describe(session)
            ),
        };
        Diagnostic::new(Level::Error, self.span, message)
    }
}

pub type PResult<T> = Result<T, ParseError>;
