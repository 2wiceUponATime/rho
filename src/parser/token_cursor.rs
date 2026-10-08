use super::*;

pub struct TokenCursor {
    tokens: Vec<Token>,
    position: u32,
}

impl TokenCursor {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    pub fn first(&self) -> Token {
        self.tokens[self.position as usize]
    }

    pub fn second(&self) -> Token {
        let i = (self.position as usize + 1).min(self.tokens.len() - 1);
        self.tokens[i]
    }

    pub fn prev(&self) -> Token {
        self.tokens[self
            .position
            .checked_sub(1)
            .expect("prev() called before bump()") as usize]
    }

    pub fn bump(&mut self) -> Token {
        if self.position < self.tokens.len() as u32 {
            self.position += 1;
        }
        self.tokens[(self.position - 1) as usize]
    }

    pub fn is_eof(&self) -> bool {
        self.first().kind == Eof
    }
}
