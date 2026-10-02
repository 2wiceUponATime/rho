#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn dummy() -> Self {
        Self::new(0, 0)
    }

    pub fn to(&self, other: Self) -> Self {
        Self::new(self.start, other.end)
    }

    pub fn range(&self) -> std::ops::Range<usize> {
        self.start as usize..self.end as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new() {
        let span = Span::new(2, 5);
        assert_eq!((span.start, span.end), (2, 5));
    }

    #[test]
    fn dummy() {
        assert_eq!(Span::dummy(), Span::new(0, 0));
    }
}
