use indexmap::IndexSet;

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
}
