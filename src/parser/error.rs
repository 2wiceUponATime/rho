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
            Self::Ident => "identifier".to_owned(),
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

    pub fn to_diag(&self, session: &ParseSession, file_id: FileId) -> Diagnostic {
        let message = match &self.kind {
            ParseErrorKind::Unexpected { expected, found } => format!(
                "expected {}{} but found {}",
                if expected.len() > 1 { "one of: " } else { "" },
                expected
                    .iter()
                    .map(|k| k.describe(session))
                    .collect::<Vec<_>>()
                    .join(", "),
                found.describe(session)
            ),
        };
        Diagnostic::new(Level::Error, file_id, self.span, message)
    }
}

pub type PResult<T> = Result<T, ParseError>;
