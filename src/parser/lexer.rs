use crate::{
    interner::Symbol,
    parser::{cursor::Cursor, lexer::TokenKind::*},
    session::{Diagnostic, FileId, Level, ParseSession},
    span::Span,
};

pub fn is_id_start(c: char) -> bool {
    unicode_ident::is_xid_start(c)
}

pub fn is_id_continue(c: char) -> bool {
    unicode_ident::is_xid_continue(c)
}

#[derive(Debug, PartialEq, Eq)]
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

#[derive(Debug)]
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
        is_id_start(c) || matches!(c, '0'..='9' | '(' | ')' | '+' | '-' | '*' | '/' | ';')
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
                '*' => self.block_comment(),
                _ => Slash,
            },
            ';' => Semi,
            _ => {
                self.cursor.eat_while(|c| !Lexer::is_known_start(c));
                Unknown
            },
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
            return FloatLiteral;
        }
        IntLiteral
    }

    fn line_comment(&mut self) -> TokenKind {
        self.cursor.eat_until(b'\n');
        LineComment
    }

    fn block_comment(&mut self) -> TokenKind {
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
        }
        BlockComment
    }

    fn error(&self, start: u32, end: u32, message: String) {
        let span = self.file_id.span(start, end);
        self.session
            .diag(Diagnostic::new(Level::Error, span, message));
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
