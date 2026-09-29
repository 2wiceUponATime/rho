use super::cursor::Cursor;
use crate::{
    interner::Symbol,
    session::{Diagnostic, FileId, Level, ParseSession},
    span::Span,
};
use TokenKind::*;

pub fn is_id_start(c: char) -> bool {
    matches!(c, '$' | '_') || unicode_ident::is_xid_start(c)
}

pub fn is_id_continue(c: char) -> bool {
    c == '$' || unicode_ident::is_xid_continue(c)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Eof,
    Unknown,
    LineComment,
    BlockComment,

    IntLiteral,
    FloatLiteral,
    StringLiteral,

    NoSubTemplate,
    TemplateHead,
    TemplateMiddle,
    TemplateTail,

    Ident(Symbol),

    /// `(`
    OpenParen,
    /// `)`
    CloseParen,
    /// `[`
    OpenBracket,
    /// `]`
    CloseBracket,
    /// `{`
    OpenBrace,
    /// `}`
    CloseBrace,
    /// `;`
    Semi,
    /// `,`
    Comma,

    /// `+`
    Plus,
    /// `-`
    Minus,
    /// `*`
    Star,
    /// `/`
    Slash,
    /// `%`
    Percent,
    /// `**`
    StarStar,
    /// `.`
    Dot,
    /// `...`
    DotDotDot,
    /// `?.`
    QuestionDot,
    /// `?`
    Question,
    /// `??`
    QuestionQuestion,
    /// `:`
    Colon,
    /// `~`
    Tilde,
    /// `!`
    Exclam,
    /// `&`
    Amp,
    /// `^`
    Caret,
    /// `|`
    Pipe,
    /// `&&`
    AmpAmp,
    /// `||`
    PipePipe,
    /// `=`
    Eq,
    /// `==`
    EqEq,
    /// `!=`
    ExclamEq,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    LtEq,
    /// `>=`
    GtEq,
    /// `<<`
    LtLt,
    /// `>>`
    GtGt,
    /// `=>`
    EqGt,
    /// `+=`
    PlusEq,
    /// `-=`
    MinusEq,
    /// `*=`
    StarEq,
    /// `/=`
    SlashEq,
    /// `%=`
    PercentEq,
    /// `**=`
    StarStarEq,
    /// `&=`
    AmpEq,
    /// `^=`
    CaretEq,
    /// `|=`
    PipeEq,
    /// `&&=`
    AmpAmpEq,
    /// `||=`
    PipePipeEq,
    /// `<<=`
    LtLtEq,
    /// `>>=`
    GtGtEq,
    /// `??=`
    QuestionQuestionEq,
}

impl TokenKind {
    pub fn describe(&self, session: &ParseSession) -> String {
        match self {
            Eof => "EOF",
            Unknown => "<error token>",
            LineComment => "line comment",
            BlockComment => "block comment",

            IntLiteral => "integer literal",
            FloatLiteral => "float literal",
            StringLiteral => "string literal",

            NoSubTemplate | TemplateHead => "template string",
            TemplateMiddle | TemplateTail => "template string continuation",

            Ident(sym) => return format!("'{}'", session.interner.borrow().get(*sym)),

            OpenParen => "'('",
            CloseParen => "')'",
            OpenBracket => "'['",
            CloseBracket => "']'",
            OpenBrace => "'{'",
            CloseBrace => "'}'",
            Semi => "';'",
            Comma => "','",

            Plus => "'+'",
            Minus => "'-'",
            Star => "'*'",
            Slash => "'/'",
            Percent => "'%'",
            StarStar => "'**'",
            Dot => "'.'",
            DotDotDot => "'...'",
            QuestionDot => "'?.'",
            Question => "'?'",
            QuestionQuestion => "'??'",
            Colon => "':'",
            Tilde => "'~'",
            Exclam => "'!'",
            Amp => "'&'",
            Caret => "'^'",
            Pipe => "'|'",
            AmpAmp => "'&&'",
            PipePipe => "'||'",
            Eq => "'='",
            EqEq => "'=='",
            ExclamEq => "'!='",
            Lt => "'<'",
            Gt => "'>'",
            LtEq => "'<='",
            GtEq => "'>='",
            LtLt => "'<<'",
            GtGt => "'>>'",
            EqGt => "'=>'",
            PlusEq => "'+='",
            MinusEq => "'-='",
            StarEq => "'*='",
            SlashEq => "'/='",
            PercentEq => "'%='",
            StarStarEq => "'**='",
            AmpEq => "'&='",
            CaretEq => "'^='",
            PipeEq => "'|='",
            AmpAmpEq => "'&&='",
            PipePipeEq => "'||='",
            LtLtEq => "'<<='",
            GtGtEq => "'>>='",
            QuestionQuestionEq => "'??='",
        }
        .into()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub span: Span,
    pub kind: TokenKind,
}

impl Token {
    pub fn new(span: Span, kind: TokenKind) -> Self {
        Self { span, kind }
    }
}

#[derive(Clone, Copy)]
struct TemplateState {
    start: u32,
    brace_depth: u32,
}

impl TemplateState {
    fn new(start: u32) -> Self {
        Self {
            start,
            brace_depth: 0,
        }
    }
}

pub struct Lexer<'psess, 'src> {
    pub session: &'psess ParseSession,
    file_id: FileId,
    src: &'src str,
    cursor: Cursor<'src>,
    template_stack: Vec<TemplateState>,
    emitted_eof: bool,
}

impl<'psess, 'src> Lexer<'psess, 'src> {
    pub fn new(file_id: FileId, session: &'psess ParseSession, src: &'src str) -> Self {
        Self {
            session,
            file_id,
            src,
            cursor: Cursor::new(src),
            template_stack: vec![],
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
            || matches!(
                c,
                '0'..='9'
                    | '"'
                    | '\''
                    | '`'
                    | '('
                    | ')'
                    | '['
                    | ']'
                    | '{'
                    | '}'
                    | ';'
                    | ','
                    | '+'
                    | '-'
                    | '*'
                    | '/'
                    | '%'
                    | '.'
                    | '?'
                    | ':'
                    | '~'
                    | '!'
                    | '&'
                    | '^'
                    | '|'
                    | '='
                    | '<'
                    | '>'
            )
    }

    #[inline]
    fn bump(&mut self, n: u8, kind: TokenKind) -> TokenKind {
        for _ in 0..n {
            self.cursor.bump();
        }
        kind
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
            '"' => self.string_literal(b'"'),
            '\'' => self.string_literal(b'\''),
            '`' => self.template_literal(start, true),
            '(' => OpenParen,
            ')' => CloseParen,
            '[' => OpenBracket,
            ']' => CloseBracket,
            '{' => match self.template_stack.last_mut() {
                Some(state) => {
                    state.brace_depth += 1;
                    OpenBrace
                }
                None => OpenBrace,
            },
            '}' => match self.template_stack.last_mut() {
                Some(&mut TemplateState {
                    brace_depth: 0,
                    start,
                }) => {
                    self.template_stack.pop();
                    self.template_literal(start, false)
                }
                Some(state) => {
                    state.brace_depth -= 1;
                    CloseBrace
                }
                None => CloseBrace,
            },
            ';' => Semi,
            ',' => Comma,

            '+' => match self.cursor.first() {
                '=' => self.bump(1, PlusEq),
                _ => Plus,
            },
            '-' => match self.cursor.first() {
                '=' => self.bump(1, MinusEq),
                _ => Minus,
            },
            '*' => match (self.cursor.first(), self.cursor.second()) {
                ('*', '=') => self.bump(2, StarStarEq),
                ('*', _) => self.bump(1, StarStar),
                ('=', _) => self.bump(1, StarEq),
                _ => Star,
            },
            '/' => match self.cursor.first() {
                '=' => self.bump(1, SlashEq),
                '/' => self.line_comment(),
                '*' => self.block_comment(start),
                _ => Slash,
            },
            '%' => match self.cursor.first() {
                '=' => self.bump(1, PercentEq),
                _ => Percent,
            },
            '.' => match (self.cursor.first(), self.cursor.second()) {
                ('.', '.') => self.bump(2, DotDotDot),
                _ => Dot,
            },
            '?' => match (self.cursor.first(), self.cursor.second()) {
                ('.', _) => self.bump(1, QuestionDot),
                ('?', '=') => self.bump(2, QuestionQuestionEq),
                ('?', _) => self.bump(1, QuestionQuestion),
                _ => Question,
            },
            ':' => Colon,
            '~' => Tilde,
            '!' => match self.cursor.first() {
                '=' => self.bump(1, ExclamEq),
                _ => Exclam,
            },
            '&' => match (self.cursor.first(), self.cursor.second()) {
                ('&', '=') => self.bump(2, AmpAmpEq),
                ('&', _) => self.bump(1, AmpAmp),
                ('=', _) => self.bump(1, AmpEq),
                _ => Amp,
            },
            '^' => match self.cursor.first() {
                '=' => self.bump(1, CaretEq),
                _ => Caret,
            },
            '|' => match (self.cursor.first(), self.cursor.second()) {
                ('|', '=') => self.bump(2, PipePipeEq),
                ('|', _) => self.bump(1, PipePipe),
                ('=', _) => self.bump(1, PipeEq),
                _ => Pipe,
            },
            '=' => match self.cursor.first() {
                '=' => self.bump(1, EqEq),
                '>' => self.bump(1, EqGt),
                _ => Eq,
            },
            '<' => match (self.cursor.first(), self.cursor.second()) {
                ('<', '=') => self.bump(2, LtLtEq),
                ('<', _) => self.bump(1, LtLt),
                ('=', _) => self.bump(1, LtEq),
                _ => Lt,
            },
            '>' => match (self.cursor.first(), self.cursor.second()) {
                ('>', '=') => self.bump(2, GtGtEq),
                ('>', _) => self.bump(1, GtGt),
                ('=', _) => self.bump(1, GtEq),
                _ => Gt,
            },
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

    fn template_literal(&mut self, start: u32, is_head: bool) -> TokenKind {
        let (end_kind, continue_kind) = if is_head {
            (NoSubTemplate, TemplateHead)
        } else {
            (TemplateTail, TemplateMiddle)
        };
        loop {
            self.cursor.eat_until3(b'`', b'\\', b'$');
            if self.cursor.is_eof() {
                self.error(start, start + 1, "Unterminated template literal".into());
                return end_kind;
            }
            let next = self.cursor.bump().unwrap();
            if next == '`' {
                return end_kind;
            }
            if next == '\\' {
                self.cursor.bump();
                continue;
            }
            if self.cursor.first() == '{' {
                self.cursor.bump();
                self.template_stack.push(TemplateState::new(start));
                return continue_kind;
            }
        }
    }

    fn string_literal(&mut self, quote: u8) -> TokenKind {
        let start = self.cursor.pos() - 1;
        loop {
            self.cursor.eat_until2(quote, b'\\');
            if self.cursor.is_eof() {
                self.error(start, start + 1, "Unterminated string literal".into());
            }
            if self.cursor.bump().unwrap() == '\\' {
                self.cursor.bump();
                continue;
            }
            break;
        }
        StringLiteral
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
        while let Some(offset) = memchr::memchr2(b'/', b'*', &bytes[i..]) {
            i += offset;
            match (bytes[i], bytes.get(i + 1)) {
                (b'/', Some(b'*')) => {
                    depth += 1;
                    i += 2;
                }
                (b'*', Some(b'/')) => {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        self.cursor.skip_bytes(i);
                        return BlockComment;
                    }
                }
                _ => i += 1,
            }
        }
        self.cursor.skip_to_end();
        self.error(start, start + 2, "Unterminated block comment".into());
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
    fn operator_tokens() {
        let (session, tokens) = lex("[ ] , . ?. ? ?? ??= : ~ ! & ^ | && || = == != \
             < > <= >= << >> += -= *= /= %= **= &= ^= |= &&= ||= <<= >>=");
        assert_eq!(
            get_kinds(&tokens),
            [
                OpenBracket,
                CloseBracket,
                Comma,
                Dot,
                QuestionDot,
                Question,
                QuestionQuestion,
                QuestionQuestionEq,
                Colon,
                Tilde,
                Exclam,
                Amp,
                Caret,
                Pipe,
                AmpAmp,
                PipePipe,
                Eq,
                EqEq,
                ExclamEq,
                Lt,
                Gt,
                LtEq,
                GtEq,
                LtLt,
                GtGt,
                PlusEq,
                MinusEq,
                StarEq,
                SlashEq,
                PercentEq,
                StarStarEq,
                AmpEq,
                CaretEq,
                PipeEq,
                AmpAmpEq,
                PipePipeEq,
                LtLtEq,
                GtGtEq,
                Eof,
            ]
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
    fn dot_only_float_literals_lex_as_dot() {
        let (session, tokens) = lex("1. .5 1.2.3");
        assert_eq!(
            get_kinds(&tokens),
            [
                // 1.
                IntLiteral,
                Dot,
                // .5
                Dot,
                IntLiteral,
                // 1.2.3
                FloatLiteral,
                Dot,
                IntLiteral,
                Eof
            ]
        );
        assert_clean(&session);
    }

    #[test]
    fn double_dot_lexes_as_two_dots() {
        let (session, tokens) = lex("1..2");
        assert_eq!(get_kinds(&tokens), [IntLiteral, Dot, Dot, IntLiteral, Eof]);
        assert_eq!(get_spans(&tokens), [(0, 1), (1, 2), (2, 3), (3, 4), (4, 4)]);
        assert_clean(&session);
    }

    #[test]
    fn dot_before_ident_lexes_as_dot() {
        let (session, tokens) = lex("1.e");
        assert_matches!(
            get_kinds(&tokens).as_slice(),
            [IntLiteral, Dot, Ident(_), Eof]
        );
        assert_clean(&session);
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
    fn error_unexpected_char_does_not_swallow_known_token() {
        let (session, tokens) = lex("@[");
        assert_eq!(get_kinds(&tokens), [Unknown, OpenBracket, Eof]);
        assert_diags(&session, &[(0, 1, UNEXPECTED_CHAR)]);
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
