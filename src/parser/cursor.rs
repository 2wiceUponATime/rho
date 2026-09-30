use std::str::Chars;

pub const EOF_CHAR: char = '\0';

pub struct Cursor<'a> {
    chars: Chars<'a>,
    input_len: u32,
}

impl<'a> Cursor<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars(),
            input_len: input.len() as u32,
        }
    }

    pub fn as_str(&self) -> &'a str {
        self.chars.as_str()
    }

    pub fn pos(&self) -> u32 {
        self.input_len - self.chars.as_str().len() as u32
    }

    #[inline]
    pub fn first(&self) -> char {
        self.chars.clone().next().unwrap_or(EOF_CHAR)
    }

    #[inline]
    pub fn second(&self) -> char {
        let mut iter = self.chars.clone();
        iter.next();
        iter.next().unwrap_or(EOF_CHAR)
    }

    pub fn is_eof(&self) -> bool {
        self.chars.as_str().is_empty()
    }

    pub fn bump(&mut self) -> Option<char> {
        self.chars.next()
    }

    pub fn skip_bytes(&mut self, n: usize) {
        self.chars = self.as_str()[n..].chars();
    }

    pub fn skip_to_end(&mut self) {
        self.chars = "".chars();
    }

    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while predicate(self.first()) && !self.is_eof() {
            self.bump();
        }
    }

    pub fn eat_until(&mut self, byte: u8) {
        self.chars = match memchr::memchr(byte, self.as_str().as_bytes()) {
            Some(index) => self.as_str()[index..].chars(),
            None => "".chars(),
        }
    }

    pub fn eat_until2(&mut self, byte1: u8, byte2: u8) {
        self.chars = match memchr::memchr2(byte1, byte2, self.as_str().as_bytes()) {
            Some(index) => self.as_str()[index..].chars(),
            None => "".chars(),
        };
    }

    pub fn eat_until3(&mut self, byte1: u8, byte2: u8, byte3: u8) {
        self.chars = match memchr::memchr3(byte1, byte2, byte3, self.as_str().as_bytes()) {
            Some(index) => self.as_str()[index..].chars(),
            None => "".chars(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_and_second() {
        let cursor = Cursor::new("ab");
        assert_eq!(cursor.first(), 'a');
        assert_eq!(cursor.second(), 'b');
    }

    #[test]
    fn first_and_second_at_eof() {
        let cursor = Cursor::new("a");
        assert_eq!(cursor.second(), EOF_CHAR);
        let cursor = Cursor::new("");
        assert_eq!(cursor.first(), EOF_CHAR);
        assert_eq!(cursor.second(), EOF_CHAR);
    }

    #[test]
    fn bump_advances() {
        let mut cursor = Cursor::new("ab");
        assert_eq!(cursor.bump(), Some('a'));
        assert_eq!(cursor.bump(), Some('b'));
        assert_eq!(cursor.bump(), None);
        assert!(cursor.is_eof());
    }

    #[test]
    fn pos_counts_bytes() {
        let mut cursor = Cursor::new("éa");
        assert_eq!(cursor.pos(), 0);
        cursor.bump();
        assert_eq!(cursor.pos(), 2);
        cursor.bump();
        assert_eq!(cursor.pos(), 3);
    }

    #[test]
    fn as_str_is_remaining_input() {
        let mut cursor = Cursor::new("abc");
        cursor.bump();
        assert_eq!(cursor.as_str(), "bc");
    }

    #[test]
    fn eat_while_stops_at_predicate() {
        let mut cursor = Cursor::new("aaab");
        cursor.eat_while(|c| c == 'a');
        assert_eq!(cursor.as_str(), "b");
    }

    #[test]
    fn eat_while_stops_at_eof() {
        let mut cursor = Cursor::new("aaa");
        cursor.eat_while(|_| true);
        assert!(cursor.is_eof());
    }

    #[test]
    fn eat_while_does_not_stop_at_literal_nul() {
        let mut cursor = Cursor::new("a\0b");
        cursor.eat_while(|_| true);
        assert!(cursor.is_eof());
    }

    #[test]
    fn eat_until_found() {
        let mut cursor = Cursor::new("abc\ndef");
        cursor.eat_until(b'\n');
        assert_eq!(cursor.as_str(), "\ndef");
        assert_eq!(cursor.pos(), 3);
    }

    #[test]
    fn eat_until_not_found() {
        let mut cursor = Cursor::new("abc");
        cursor.eat_until(b'\n');
        assert!(cursor.is_eof());
        assert_eq!(cursor.pos(), 3);
    }

    #[test]
    fn skip_bytes() {
        let mut cursor = Cursor::new("éab");
        cursor.skip_bytes(2);
        assert_eq!(cursor.as_str(), "ab");
        assert_eq!(cursor.pos(), 2);
    }

    #[test]
    fn skip_to_end() {
        let mut cursor = Cursor::new("abc");
        cursor.skip_to_end();
        assert!(cursor.is_eof());
        assert_eq!(cursor.pos(), 3);
    }
}
