use indexmap::IndexSet;
use unicode_normalization::{IsNormalized, UnicodeNormalization};
use kw::Keyword;

macro_rules! keywords {
    ($($set:ident { $($name:ident: $str:literal,)* })*) => {
        #[allow(non_upper_case_globals)]
        pub mod kw {
            use super::Symbol;

            #[repr(u32)]
            pub enum Keyword {
                $($($name,)* )*
            }

            impl Keyword {
                pub const STRINGS: &[&str] = &[$($($str,)*)*];
                pub const COUNT: u32 = Self::STRINGS.len() as u32;
            }

            $($(pub const $name: Symbol = Symbol(Keyword::$name as u32);)*)*

            pub const KEYWORDS: &[(Symbol, &str)] = &[$($(($name, $str),)*)*];
        }

        impl KwSet {
            $(pub const $set: KwSet = KwSet::of(&[$(kw::$name,)*]);)*
        }
    }
}

keywords! {
    STRICT {
        SelfType: "Self",
        Underscore: "_",
        Async: "async",
        Await: "await",
        Break: "break",
        Catch: "catch",
        Class: "class",
        Const: "const",
        Continue: "continue",
        Do: "do",
        Else: "else",
        False: "false",
        Finally: "finally",
        For: "for",
        Function: "function",
        If: "if",
        In: "in",
        Is: "is",
        Let: "let",
        Loop: "loop",
        Match: "match",
        Null: "null",
        Return: "return",
        Super: "super",
        This: "this",
        Throw: "throw",
        True: "true",
        Try: "try",
        While: "while",
    }
    CONTEXTUAL {
        Constructor: "constructor",
        Extends: "extends",
        Private: "private",
        Protected: "protected",
        Static: "static",
    }
}

pub struct Interner {
    set: IndexSet<Box<str>>,
}

impl Default for Interner {
    fn default() -> Self {
        let mut result = Self {
            set: IndexSet::new(),
        };
        for keyword in Keyword::STRINGS {
            result.intern(keyword);
        }
        result
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Symbol(u32);

#[derive(Clone, Copy)]
pub struct KwSet(u64);

impl KwSet {
    pub const EMPTY: Self = Self(0);

    pub const fn of(kws: &[Symbol]) -> Self {
        let mut bits = 0;
        let mut i = 0;
        while i < kws.len() {
            bits |= 1 << kws[i].0;
            i += 1;
        }
        Self(bits)
    }

    pub const fn with(&self, sym: Symbol) -> Self {
        if sym.0 >= Keyword::COUNT {
            *self
        } else {
            Self(self.0 & (1 << sym.0))
        }
    }

    pub const fn union(&self, other: &KwSet) -> Self {
        Self(self.0 | other.0)
    }

    #[inline]
    pub fn contains(&self, sym: Symbol) -> bool {
        sym.0 < Keyword::COUNT && self.0 & (1 << sym.0) != 0
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

    #[test]
    fn new_interner_contains_only_keywords() {
        let interner = Interner::new();
        assert_eq!(interner.set.len(), Keyword::COUNT as usize);
    }

    #[test]
    fn keywords_are_preinterned() {
        let mut interner = Interner::new();
        for (i, &(sym, s)) in kw::KEYWORDS.iter().enumerate() {
            assert_eq!(sym.0, i as u32, "`{s}` has the wrong symbol ID");
            assert_eq!(interner.intern(s), sym, "interning `{s}`");
            assert_eq!(interner.get(sym), s, "resolving `{s}`");
        }
    }
}
