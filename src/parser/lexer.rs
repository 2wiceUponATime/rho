use crate::{
    interner::Symbol,
    parser::{cursor::Cursor, lexer::TokenKind::*},
    session::{Diagnostic, FileId, Level, ParseSession},
    span::Span,
};

pub fn is_id_start(c: char) -> bool {
    matches!(c, '$' | '_') || unicode_ident::is_xid_start(c)
}

pub fn is_id_continue(c: char) -> bool {
    c == '$' || unicode_ident::is_xid_continue(c)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Ident(Symbol),

    IntLiteral,
    FloatLiteral,

    LineComment,
    BlockComment,

    OpenParen,
    CloseParen,
    Plus,
    Minus,
    Star,
    Slash,
    Semi,

    Eof,
    Unknown,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub span: Span,
    pub kind: TokenKind,
}

impl Token {
    pub fn new(span: Span, kind: TokenKind) -> Self {
        Self { span, kind }
    }
}

pub struct Lexer<'psess, 'src> {
    session: &'psess ParseSession,
    file_id: FileId,
    src: &'src str,
    cursor: Cursor<'src>,
    emitted_eof: bool,
}

impl<'psess, 'src> Lexer<'psess, 'src> {
    pub fn new(file_id: FileId, session: &'psess ParseSession, src: &'src str) -> Self {
        Self {
            session,
            file_id,
            src,
            cursor: Cursor::new(src),
            emitted_eof: false,
        }
    }

    pub fn advance_token(&mut self) -> Token {
        self.cursor.eat_while(char::is_whitespace);
        let start = self.cursor.pos();
        let kind = self.lex_token_kind(start);
        let end = self.cursor.pos();
        if kind == Eof {
            self.emitted_eof = true;
        }
        if kind == Unknown {
            self.error(start, end, "Unexpected character".into());
        }
        Token::new(self.file_id.span(start, end), kind)
    }

    fn is_known_start(c: char) -> bool {
        c.is_whitespace()
            || is_id_start(c)
            || matches!(c, '0'..='9' | '(' | ')' | '+' | '-' | '*' | '/' | ';')
    }

    fn lex_token_kind(&mut self, start: u32) -> TokenKind {
        let Some(c) = self.cursor.bump() else {
            return Eof;
        };
        if is_id_start(c) {
            return self.ident(start);
        }
        match c {
            '0'..='9' => self.number_literal(),
            '(' => OpenParen,
            ')' => CloseParen,
            '+' => Plus,
            '-' => Minus,
            '*' => Star,
            '/' => match self.cursor.first() {
                '/' => self.line_comment(),
                '*' => self.block_comment(start),
                _ => Slash,
            },
            ';' => Semi,
            _ => {
                self.cursor.eat_while(|c| !Lexer::is_known_start(c));
                Unknown
            }
        }
    }

    fn ident(&mut self, start: u32) -> TokenKind {
        self.cursor.eat_while(is_id_continue);
        let value = &self.src[start as usize..self.cursor.pos() as usize];
        Ident(self.session.interner.borrow_mut().intern(value))
    }

    fn number_literal(&mut self) -> TokenKind {
        self.cursor.eat_while(|c| c.is_ascii_digit());
        if self.cursor.first() == '.' && self.cursor.second().is_ascii_digit() {
            self.cursor.bump();
            self.cursor.eat_while(|c| c.is_ascii_digit());
            self.check_ident_after_literal("Unexpected identifier in float literal");
            return FloatLiteral;
        }
        self.check_ident_after_literal("Unexpected identifier in integer literal");
        IntLiteral
    }

    fn check_ident_after_literal(&self, message: &str) {
        let c = self.cursor.first();
        if is_id_start(c) {
            let pos = self.cursor.pos();
            self.error(pos, pos + c.len_utf8() as u32, message.into());
        }
    }

    fn line_comment(&mut self) -> TokenKind {
        self.cursor.eat_until(b'\n');
        LineComment
    }

    fn block_comment(&mut self, start: u32) -> TokenKind {
        self.cursor.bump();
        let bytes = self.cursor.as_str().as_bytes();
        let mut depth = 1usize;
        let mut i = 0usize;
        let mut closed = false;
        while let Some(current) = bytes.get(i) {
            let Some(next) = bytes.get(i + 1) else {
                break;
            };
            match (*current, *next) {
                (b'/', b'*') => {
                    depth += 1;
                    i += 2;
                }
                (b'*', b'/') => {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        self.cursor.skip_bytes(i);
                        closed = true;
                        break;
                    }
                }
                _ => i += 1,
            }
        }
        if !closed {
            self.cursor.skip_to_end();
            self.error(start, start + 2, "Unterminated block comment".into());
        }
        BlockComment
    }

    fn error(&self, start: u32, end: u32, message: String) {
        let span = self.file_id.span(start, end);
        self.session
            .diagnostics
            .borrow_mut()
            .push(Diagnostic::new(Level::Error, span, message));
    }
}

impl<'psess, 'src> Iterator for Lexer<'psess, 'src> {
    type Item = Token;

    fn next(&mut self) -> Option<Token> {
        if self.emitted_eof {
            return None;
        }
        Some(self.advance_token())
    }
}

impl std::iter::FusedIterator for Lexer<'_, '_> {}

#[cfg(test)]
mod tests {
    use std::assert_matches;

    use super::*;
    use crate::session::{FilePath, ParseSession, SourceFile};

    const UNEXPECTED_CHAR: &str = "Unexpected character";
    const IDENT_IN_INT: &str = "Unexpected identifier in integer literal";
    const IDENT_IN_FLOAT: &str = "Unexpected identifier in float literal";
    const UNTERMINATED_BLOCK: &str = "Unterminated block comment";

    fn lex(src: &str) -> (ParseSession, Vec<Token>) {
        let mut session = ParseSession::new();
        let id = session.add_source(SourceFile::new(
            FilePath::Virtual("<test>".into()),
            src.into(),
        ));
        let lexer = Lexer::new(id, &session, &session.source(id).text);
        let tokens: Vec<Token> = lexer.into_iter().collect();
        (session, tokens)
    }

    fn get_kinds(tokens: &[Token]) -> Vec<TokenKind> {
        tokens.iter().map(|t| t.kind).collect()
    }

    fn get_spans(tokens: &[Token]) -> Vec<(u32, u32)> {
        tokens.iter().map(|t| (t.span.start, t.span.end)).collect()
    }

    fn get_idents(session: &ParseSession, tokens: &[Token]) -> Vec<String> {
        let interner = session.interner.borrow();
        tokens
            .iter()
            .filter_map(|t| match t.kind {
                Ident(symbol) => Some(interner.get(symbol).to_string()),
                _ => None,
            })
            .collect()
    }

    fn assert_diags(session: &ParseSession, expected: &[(u32, u32, &str)]) {
        let diags = session.diagnostics.borrow();
        let actual: Vec<(u32, u32, &str)> = diags
            .iter()
            .map(|d| (d.span.start, d.span.end, d.message.as_str()))
            .collect();
        assert_eq!(actual, expected);
    }

    fn assert_clean(session: &ParseSession) {
        assert_diags(session, &[]);
    }

    #[test]
    fn idents() {
        let (session, tokens) = lex("foo _bar_ $baz$");
        assert_matches!(
            get_kinds(&tokens).as_slice(),
            [Ident(_), Ident(_), Ident(_), Eof]
        );
        assert_eq!(get_idents(&session, &tokens), ["foo", "_bar_", "$baz$"]);
        assert_clean(&session);
    }

    #[test]
    fn unicode_idents() {
        let (session, tokens) = lex("café 日本");
        assert_eq!(get_idents(&session, &tokens), ["café", "日本"]);
        assert_eq!(get_spans(&tokens), [(0, 5), (6, 12), (12, 12)]);
        assert_clean(&session);
    }

    #[test]
    fn same_ident_same_symbol() {
        let (session, tokens) = lex("foo foo bar");
        assert_eq!(tokens[0].kind, tokens[1].kind);
        assert_ne!(tokens[0].kind, tokens[2].kind);
        assert_clean(&session);
    }

    #[test]
    fn nfc_equivalent_idents_same_symbol() {
        let (session, tokens) = lex("caf\u{e9} cafe\u{301}");
        assert_eq!(tokens[0].kind, tokens[1].kind);
        assert_eq!(get_idents(&session, &tokens), ["caf\u{e9}", "caf\u{e9}"]);
        assert_clean(&session);
    }

    #[test]
    fn int_literals() {
        let (session, tokens) = lex("1 2 3");
        assert_matches!(
            get_kinds(&tokens).as_slice(),
            [IntLiteral, IntLiteral, IntLiteral, Eof]
        );
        assert_clean(&session);
    }

    #[test]
    fn float_literals() {
        let (session, tokens) = lex("1.0 2.0 3.0");
        assert_eq!(
            get_kinds(&tokens),
            [FloatLiteral, FloatLiteral, FloatLiteral, Eof]
        );
        assert_clean(&session);
    }

    #[test]
    fn line_comments() {
        let (session, tokens) = lex("// Line comment
identifier");
        assert_matches!(get_kinds(&tokens).as_slice(), [LineComment, Ident(_), Eof]);
        assert_clean(&session);
    }

    #[test]
    fn line_comment_span_excludes_newline() {
        let (session, tokens) = lex("// hi\nx");
        assert_eq!(get_spans(&tokens)[0], (0, 5));
        assert_clean(&session);
    }

    #[test]
    fn line_comment_at_eof() {
        let (session, tokens) = lex("// hi");
        assert_eq!(get_kinds(&tokens), [LineComment, Eof]);
        assert_eq!(get_spans(&tokens), [(0, 5), (5, 5)]);
        assert_clean(&session);
    }

    #[test]
    fn block_comments() {
        let (session, tokens) = lex("/* Block comment */ foo
/* Nested /* block /* comment */ */ */ bar");
        assert_matches!(
            get_kinds(&tokens).as_slice(),
            [BlockComment, Ident(_), BlockComment, Ident(_), Eof]
        );
        assert_clean(&session);
    }

    #[test]
    fn empty_block_comments() {
        let (session, tokens) = lex("/**/");
        assert_eq!(get_kinds(&tokens), [BlockComment, Eof]);
        assert_eq!(get_spans(&tokens), [(0, 4), (4, 4)]);
        assert_clean(&session);

        let (session, tokens) = lex("/***/");
        assert_eq!(get_kinds(&tokens), [BlockComment, Eof]);
        assert_eq!(get_spans(&tokens), [(0, 5), (5, 5)]);
        assert_clean(&session);
    }

    #[test]
    fn simple_tokens() {
        let (session, tokens) = lex("( ) + - * / ;");
        assert_eq!(
            get_kinds(&tokens),
            [OpenParen, CloseParen, Plus, Minus, Star, Slash, Semi, Eof]
        );
        assert_clean(&session);
    }

    #[test]
    fn eof() {
        let (session, tokens) = lex("   ");
        assert_eq!(get_kinds(&tokens), [Eof]);
        assert_eq!(get_spans(&tokens), [(3, 3)]);
        assert_clean(&session);
    }

    #[test]
    fn empty_input() {
        let (session, tokens) = lex("");
        assert_eq!(get_kinds(&tokens), [Eof]);
        assert_eq!(get_spans(&tokens), [(0, 0)]);
        assert_clean(&session);
    }

    #[test]
    fn iterator_fused_after_eof() {
        let mut session = ParseSession::new();
        let id = session.add_source(SourceFile::new(
            FilePath::Virtual("<test>".into()),
            "x".into(),
        ));
        let mut lexer = Lexer::new(id, &session, &session.source(id).text);
        assert_matches!(lexer.next().map(|t| t.kind), Some(Ident(_)));
        assert_eq!(lexer.next().map(|t| t.kind), Some(Eof));
        assert_eq!(lexer.next(), None);
        assert_eq!(lexer.next(), None);
    }

    #[test]
    fn error_bad_float_literals() {
        let (session, tokens) = lex("1. .5 1.2.3");
        assert_eq!(
            get_kinds(&tokens),
            [
                // 1.
                IntLiteral,
                Unknown,
                // .5
                Unknown,
                IntLiteral,
                // 1.2.3
                FloatLiteral,
                Unknown,
                IntLiteral,
                Eof
            ]
        );
        assert_diags(
            &session,
            &[
                (1, 2, UNEXPECTED_CHAR),
                (3, 4, UNEXPECTED_CHAR),
                (9, 10, UNEXPECTED_CHAR),
            ],
        );
    }

    #[test]
    fn error_double_dot() {
        let (session, tokens) = lex("1..2");
        assert_eq!(get_kinds(&tokens), [IntLiteral, Unknown, IntLiteral, Eof]);
        assert_eq!(get_spans(&tokens), [(0, 1), (1, 3), (3, 4), (4, 4)]);
        assert_diags(&session, &[(1, 3, UNEXPECTED_CHAR)]);
    }

    #[test]
    fn error_dot_before_ident() {
        let (session, tokens) = lex("1.e");
        assert_matches!(
            get_kinds(&tokens).as_slice(),
            [IntLiteral, Unknown, Ident(_), Eof]
        );
        assert_diags(&session, &[(1, 2, UNEXPECTED_CHAR)]);
    }

    #[test]
    fn error_unexpected_chars() {
        let (session, tokens) = lex("@ @@@");
        assert_eq!(get_kinds(&tokens), [Unknown, Unknown, Eof]);
        assert_diags(
            &session,
            &[(0, 1, UNEXPECTED_CHAR), (2, 5, UNEXPECTED_CHAR)],
        );
    }

    #[test]
    fn error_unexpected_multibyte_chars() {
        let (session, tokens) = lex("€€ a");
        assert_matches!(get_kinds(&tokens).as_slice(), [Unknown, Ident(_), Eof]);
        assert_eq!(get_spans(&tokens)[0], (0, 6));
        assert_diags(&session, &[(0, 6, UNEXPECTED_CHAR)]);
    }

    #[test]
    fn error_unterminated_block_comment() {
        let (session, tokens) = lex("/*");
        assert_eq!(get_kinds(&tokens), [BlockComment, Eof]);
        assert_diags(&session, &[(0, 2, UNTERMINATED_BLOCK)]);
    }

    #[test]
    fn error_unterminated_block_comment_points_at_opener() {
        let (session, tokens) = lex("x /* comment");
        assert_eq!(get_spans(&tokens)[1], (2, 12));
        assert_diags(&session, &[(2, 4, UNTERMINATED_BLOCK)]);
    }

    #[test]
    fn error_unterminated_nested_block_comment_points_at_outer_opener() {
        let (session, _) = lex("/* /* */");
        assert_diags(&session, &[(0, 2, UNTERMINATED_BLOCK)]);
    }

    #[test]
    fn error_unterminated_block_comment_ending_in_multibyte() {
        let (session, _) = lex("/* é");
        assert_diags(&session, &[(0, 2, UNTERMINATED_BLOCK)]);
    }

    #[test]
    fn error_ident_in_number_literal() {
        let (session, tokens) = lex("123abc 1.23abc");
        assert_matches!(
            get_kinds(&tokens).as_slice(),
            [IntLiteral, Ident(_), FloatLiteral, Ident(_), Eof]
        );
        assert_diags(&session, &[(3, 4, IDENT_IN_INT), (11, 12, IDENT_IN_FLOAT)]);
    }

    #[test]
    fn error_multibyte_ident_in_number_literal() {
        let (session, _) = lex("1é 1.5é");
        assert_diags(&session, &[(1, 3, IDENT_IN_INT), (7, 9, IDENT_IN_FLOAT)]);
    }

    #[test]
    fn spans() {
        let (_, tokens) = lex("1 two 3");
        assert_eq!(get_spans(&tokens), [(0, 1), (2, 5), (6, 7), (7, 7)]);
    }

    #[test]
    fn whitespace_skipped() {
        let (_, tokens) = lex("   foo");
        assert_matches!(get_spans(&tokens).as_slice(), [(3, _), _]);
    }

    #[test]
    fn spans_on_char_boundaries() {
        let inputs = [
            "café 日本",
            "cafe\u{301}",
            "/* é",
            "/* /* é */",
            "// é",
            "1é",
            "1.5é",
            "€€ é",
            "é€",
            "a\r\nb",
            "\u{a0}x",
        ];
        for src in inputs {
            let (session, tokens) = lex(src);
            let diags = session.diagnostics.borrow();
            let spans = tokens
                .iter()
                .map(|t| t.span)
                .chain(diags.iter().map(|d| d.span));
            for span in spans {
                assert!(
                    src.is_char_boundary(span.start as usize)
                        && src.is_char_boundary(span.end as usize),
                    "span {}..{} not on char boundary in {src:?}",
                    span.start,
                    span.end
                );
                session.display_span(span);
            }
            for diag in diags.iter() {
                session.display_diag(diag);
            }
        }
    }
}
