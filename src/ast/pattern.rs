use super::*;

pub enum Pattern {
    Variable(Symbol),
}

impl Pattern {
    pub fn display(&self, session: &ParseSession) -> impl fmt::Display {
        fmt::from_fn(|f| Printer::new(f, session).pattern(self))
    }
}

assert_size!(Pattern, 4);
