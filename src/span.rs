use crate::session::FileId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub file_id: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(file_id: FileId, start: u32, end: u32) -> Self {
        Self {
            file_id,
            start,
            end,
        }
    }

    pub fn dummy() -> Self {
        Self::new(FileId(0), 0, 0)
    }

    pub fn to(&self, other: Self) -> Self {
        Self::new(self.file_id, self.start, other.end)
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
        let span = Span::new(FileId(1), 2, 5);
        assert_eq!(span.file_id, FileId(1));
        assert_eq!((span.start, span.end), (2, 5));
    }

    #[test]
    fn dummy() {
        assert_eq!(Span::dummy(), Span::new(FileId(0), 0, 0));
    }

    #[test]
    fn file_id_span() {
        assert_eq!(FileId(3).span(1, 4), Span::new(FileId(3), 1, 4));
    }
}
