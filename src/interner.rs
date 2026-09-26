use indexmap::IndexSet;
use unicode_normalization::{IsNormalized, UnicodeNormalization};

#[derive(Default)]
pub struct Interner {
    set: IndexSet<Box<str>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Symbol(u32);

impl Interner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, s: &str) -> Symbol {
        let s = if s.is_ascii()
            || unicode_normalization::is_nfc_quick(s.chars()) == IsNormalized::Yes
        {
            s
        } else {
            &s.nfc().collect::<String>()
        };
        if let Some(i) = self.set.get_index_of(s) {
            return Symbol(i as u32);
        }
        let (i, _) = self.set.insert_full(s.into());
        Symbol(i as u32)
    }

    pub fn get(&self, symbol: Symbol) -> &str {
        &self.set[symbol.0 as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_string_interns_to_same_symbol() {
        let mut interner = Interner::new();
        assert_eq!(interner.intern("foo"), interner.intern("foo"));
    }

    #[test]
    fn different_strings_intern_to_different_symbols() {
        let mut interner = Interner::new();
        assert_ne!(interner.intern("foo"), interner.intern("bar"));
    }

    #[test]
    fn symbol_resolves_to_original_string() {
        let mut interner = Interner::new();
        let symbol = interner.intern("foo");
        assert_eq!(interner.get(symbol), "foo");
    }

    #[test]
    fn nfc_normalization() {
        let nfc = "\u{e9}";
        let not_nfc = "e\u{301}";
        let mut interner = Interner::new();
        let sym_nfc = interner.intern(nfc);
        let sym_not_nfc = interner.intern(not_nfc);
        assert_eq!(sym_nfc, sym_not_nfc);
        assert_eq!(interner.get(sym_nfc), nfc);
    }

    #[test]
    fn already_nfc_non_ascii() {
        let mut interner = Interner::new();
        let symbol = interner.intern("日本語");
        assert_eq!(interner.get(symbol), "日本語");
        assert_eq!(interner.intern("日本語"), symbol);
    }

    #[test]
    fn non_nfc_resolves_to_nfc() {
        let mut interner = Interner::new();
        let symbol = interner.intern("e\u{301}");
        assert_eq!(interner.get(symbol), "\u{e9}");
    }

    #[test]
    fn symbols_stable_after_more_interning() {
        let mut interner = Interner::new();
        let foo = interner.intern("foo");
        for i in 0..100 {
            interner.intern(&format!("s{i}"));
        }
        assert_eq!(interner.intern("foo"), foo);
        assert_eq!(interner.get(foo), "foo");
    }

    #[test]
    fn empty_string() {
        let mut interner = Interner::new();
        let symbol = interner.intern("");
        assert_eq!(interner.get(symbol), "");
        assert_ne!(interner.intern("a"), symbol);
    }
}
