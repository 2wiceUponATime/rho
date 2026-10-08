use std::fmt::Debug;

use super::*;

#[derive(Clone, Copy)]
pub struct Partner(u32);

impl Partner {
    const SYNTHETIC: u32 = 1 << 31;
    pub const UNMATCHED: u32 = u32::MAX;

    pub const fn real(i: u32) -> Self {
        Self(i)
    }

    pub const fn before(i: u32) -> Self {
        Self(i | Self::SYNTHETIC)
    }

    pub const fn unmatched() -> Self {
        Self(Self::UNMATCHED)
    }

    pub fn kind(&self) -> PartnerKind {
        match self.0 {
            Self::UNMATCHED => PartnerKind::Unmatched,
            x if x & Self::SYNTHETIC != 0 => PartnerKind::BeforeToken(x & !Self::SYNTHETIC),
            x => PartnerKind::Token(x),
        }
    }
}

impl Debug for Partner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind().fmt(f)
    }
}

#[derive(Debug)]
pub enum PartnerKind {
    Token(u32),
    BeforeToken(u32),
    Unmatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimKind {
    /// `()`
    Parens,
    /// `[]`
    Brackets,
    /// `{}`
    Braces,
    /// `TemplateHead` and `TemplateTail`
    Template,
}

impl DelimKind {
    pub fn describe_open(&self) -> &'static str {
        match self {
            Self::Parens => "'('",
            Self::Brackets => "'['",
            Self::Braces => "'{'",
            Self::Template => "template",
        }
    }

    pub fn describe_close(&self) -> &'static str {
        match self {
            Self::Parens => "')'",
            Self::Brackets => "']'",
            Self::Braces => "'}'",
            Self::Template => "template",
        }
    }
}

#[derive(Debug)]
struct Delim {
    kind: DelimKind,
    index: usize,
}

impl Delim {
    pub fn new(kind: DelimKind, index: usize) -> Self {
        Self { kind, index }
    }
}

fn unclosed(file_id: FileId, token: &Token, open: DelimKind) -> Diagnostic {
    Diagnostic::new(
        Level::Error,
        file_id,
        token.span,
        format!("Unclosed {}", open.describe_open()),
    )
}

fn missing_opener(file_id: FileId, token: &Token, close: DelimKind) -> Diagnostic {
    Diagnostic::new(
        Level::Error,
        file_id,
        token.span,
        format!(
            "{} missing its opening {}",
            close.describe_close(),
            close.describe_open()
        ),
    )
}

pub fn match_delims(session: &mut ParseSession, file_id: FileId, tokens: &[Token]) -> Vec<Partner> {
    let mut result = vec![Partner::unmatched(); tokens.len() - 1];
    let mut stack: Vec<Delim> = vec![];
    for (i, token) in tokens.iter().enumerate() {
        if let Some(open) = token.kind.open_delim() {
            stack.push(Delim::new(open, i));
            continue;
        }
        let Some(close) = token.kind.close_delim() else {
            continue;
        };
        match stack.last() {
            Some(open) => {
                if open.kind != close {
                    if open.kind == DelimKind::Template {
                        session
                            .diagnostics
                            .borrow_mut()
                            .push(missing_opener(file_id, token, close));
                        continue;
                    }
                    if close == DelimKind::Template {
                        let mut diags = session.diagnostics.borrow_mut();
                        loop {
                            // `unwrap()` is safe because every TemplateTail has a matching TemplateHead
                            let open = stack.pop().unwrap();
                            if open.kind == DelimKind::Template {
                                result[i] = Partner::real(open.index as u32);
                                result[open.index] = Partner::real(i as u32);
                                break;
                            }
                            diags.push(unclosed(file_id, &tokens[open.index], open.kind));
                        }
                        continue;
                    }
                    let mut insertion_branch =
                        Branch::new(tokens, i + 1, &stack[stack.len() - 1..]);
                    let mut substitution_branch = Branch::new(
                        tokens,
                        i + 1,
                        &stack[stack.len().saturating_sub(2)..stack.len() - 1],
                    );
                    while !(insertion_branch.done && substitution_branch.done) {
                        insertion_branch.step();
                        substitution_branch.step();
                    }
                    if substitution_branch.errors > insertion_branch.errors {
                        continue;
                    }
                }
                result[i] = Partner::real(open.index as u32);
                result[open.index] = Partner::real(i as u32);
                stack.pop();
            }
            None => session
                .diagnostics
                .borrow_mut()
                .push(missing_opener(file_id, token, close)),
        }
    }
    if !stack.is_empty() {
        let mut diags = session.diagnostics.borrow_mut();
        for open in stack {
            diags.push(unclosed(file_id, &tokens[open.index], open.kind));
        }
    }
    result
}

struct Branch<'a> {
    tokens: &'a [Token],
    index: usize,
    stack: &'a [Delim],
    stack_pushed: Vec<DelimKind>,
    done: bool,
    errors: u32,
}

impl<'a> Branch<'a> {
    pub fn new(tokens: &'a [Token], index: usize, stack: &'a [Delim]) -> Self {
        Self {
            tokens,
            index,
            stack,
            stack_pushed: vec![],
            done: false,
            errors: 0,
        }
    }

    fn push(&mut self, kind: DelimKind) {
        self.stack_pushed.push(kind);
    }

    fn pop(&mut self) {
        if self.stack_pushed.pop().is_none() {
            self.done = true;
        }
    }

    fn peek(&self) -> Option<&DelimKind> {
        match self.stack_pushed.last() {
            None => self.stack.last().map(|d| &d.kind),
            some => some,
        }
    }

    fn stack_len(&self) -> usize {
        self.stack_pushed.len() + self.stack.len() - self.done as usize
    }

    fn next_token(&mut self) {
        let token = self.tokens[self.index];
        self.index += 1;
        if let Some(kind) = token.kind.open_delim() {
            self.push(kind);
            return;
        }
        let Some(close) = token.kind.close_delim() else {
            return;
        };
        let Some(&open) = self.peek() else {
            self.errors += 1;
            return;
        };
        if open == close {
            self.pop();
        } else {
            self.errors += 1;
        }
    }

    pub fn step(&mut self) {
        if self.done {
            return;
        }
        self.next_token();
        let stack_len = self.stack_len();
        if self.index >= self.tokens.len() {
            self.done = true;
        }
        if self.done {
            self.errors += stack_len as u32;
        }
    }
}
