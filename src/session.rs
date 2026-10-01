use std::{cell::RefCell, fs, io, ops::Deref, path::PathBuf};

use crate::{interner::Interner, span::Span};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileId(pub u32);

impl FileId {
    pub fn span(&self, start: u32, end: u32) -> Span {
        Span::new(*self, start, end)
    }
}

pub enum FilePath {
    Real(PathBuf),
    Virtual(String),
}

pub struct SourceFile {
    pub path: FilePath,
    pub text: String,
    pub line_starts: Vec<u32>,
}

impl SourceFile {
    pub fn new(path: FilePath, text: String) -> Self {
        let bytes = text.as_bytes();
        let mut line_starts = Vec::with_capacity(bytes.len() / 32 + 1);
        line_starts.push(0);
        line_starts.extend(memchr::memchr_iter(b'\n', bytes).map(|idx| (idx + 1) as u32));
        Self {
            path,
            text,
            line_starts,
        }
    }

    pub fn from_file(path: PathBuf) -> io::Result<Self> {
        let text = fs::read_to_string(&path)?;
        Ok(Self::new(FilePath::Real(path), text))
    }

    pub fn to_pos(&self, offset: u32) -> (u32, u32) {
        let row = self.line_start_idx(offset);
        let line_start = self.line_starts[row as usize];
        let col = self.text[line_start as usize..offset as usize]
            .chars()
            .count() as u32;
        (row, col)
    }

    fn line_start_idx(&self, offset: u32) -> u32 {
        match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx as u32,
            Err(next) => (next - 1) as u32,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub enum Level {
    Warning,
    Error,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub level: Level,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn new(level: Level, span: Span, message: String) -> Self {
        Self {
            level,
            span,
            message,
        }
    }
}

#[derive(Default)]
pub struct Diagnostics {
    pub diagnostics: Vec<Diagnostic>,
    pub max_level: Option<Level>,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diag: Diagnostic) {
        let level = diag.level;
        self.diagnostics.push(diag);
        self.max_level = Some(match self.max_level {
            Some(prev) => prev.max(level),
            None => level,
        })
    }
}

impl Deref for Diagnostics {
    type Target = Vec<Diagnostic>;

    fn deref(&self) -> &Self::Target {
        &self.diagnostics
    }
}

#[derive(Default)]
pub struct ParseSession {
    source_files: Vec<SourceFile>,
    pub interner: RefCell<Interner>,
    pub diagnostics: RefCell<Diagnostics>,
}

impl ParseSession {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_source(&mut self, source_file: SourceFile) -> FileId {
        self.source_files.push(source_file);
        FileId(self.source_files.len() as u32 - 1)
    }

    pub fn add_source_file(&mut self, path: PathBuf) -> io::Result<FileId> {
        Ok(self.add_source(SourceFile::from_file(path)?))
    }

    pub fn source(&self, id: FileId) -> &SourceFile {
        &self.source_files[id.0 as usize]
    }

    pub fn text(&self, span: Span) -> &str {
        &self.source(span.file_id).text[span.range()]
    }

    pub fn char(&self, file_id: FileId, index: u32) -> char {
        self.source(file_id).text.as_bytes()[index as usize] as char
    }

    pub fn display_span(&self, span: Span) -> String {
        let source = self.source(span.file_id);
        let file: &String = match &source.path {
            FilePath::Real(path) => &path.to_str().unwrap().into(),
            FilePath::Virtual(name) => name,
        };
        let (start_row, start_col) = source.to_pos(span.start);
        let (end_row, end_col) = source.to_pos(span.end);
        format!(
            "{file}:{}:{}-{}:{}",
            start_row + 1,
            start_col + 1,
            end_row + 1,
            end_col + 1
        )
    }

    pub fn display_span_start(&self, span: Span) -> String {
        let source = self.source(span.file_id);
        let file: &String = match &source.path {
            FilePath::Real(path) => &path.to_str().unwrap().into(),
            FilePath::Virtual(name) => name,
        };
        let (row, col) = source.to_pos(span.start);
        format!("{file}:{}:{}", row + 1, col + 1,)
    }

    pub fn display_diag(&self, diag: &Diagnostic) -> String {
        format!(
            "{:?}: {} at {}",
            diag.level,
            diag.message,
            self.display_span_start(diag.span)
        )
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn virtual_source(name: &str, text: &str) -> SourceFile {
        SourceFile::new(FilePath::Virtual(name.into()), text.into())
    }

    fn dummy_diag(level: Level) -> Diagnostic {
        Diagnostic::new(level, Span::dummy(), format!("{:?}", level))
    }

    #[test]
    fn empty_source_pos() {
        let source = virtual_source("<test>", "");
        assert_eq!(source.to_pos(0), (0, 0));
    }

    #[test]
    fn start_of_line_pos() {
        let source = virtual_source("<test>", "foo\nbar");
        assert_eq!(source.to_pos(4), (1, 0));
    }

    #[test]
    fn end_of_line_pos() {
        let source = virtual_source("<test>", "foo\nbar");
        assert_eq!(source.to_pos(3), (0, 3));
    }

    #[test]
    fn end_of_source_pos() {
        let source = virtual_source("<test>", "foo");
        assert_eq!(source.to_pos(3), (0, 3));
    }

    #[test]
    fn end_of_source_pos_with_trailing_newline() {
        let source = virtual_source("<test>", "foo\n");
        assert_eq!(source.to_pos(4), (1, 0));
    }

    #[test]
    fn display_span() {
        let mut session = ParseSession::new();
        let id = session.add_source(virtual_source("<test>", "foo bar"));
        assert_eq!(session.display_span(Span::new(id, 0, 3)), "<test>:1:1-1:4");
    }

    #[test]
    fn display_span_to_end() {
        let mut session = ParseSession::new();
        let id = session.add_source(virtual_source("<test>", "foo bar"));
        assert_eq!(session.display_span(Span::new(id, 0, 7)), "<test>:1:1-1:8");
    }

    #[test]
    fn display_span_multiline() {
        let mut session = ParseSession::new();
        let id = session.add_source(virtual_source("<test>", "foo\nbar"));
        assert_eq!(session.display_span(Span::new(id, 0, 5)), "<test>:1:1-2:2");
    }

    #[test]
    fn diagnostics_max_level() {
        let mut diags = Diagnostics::new();
        assert_eq!(diags.max_level, None);

        diags.push(dummy_diag(Level::Warning));
        assert_eq!(diags.max_level, Some(Level::Warning));

        diags.push(dummy_diag(Level::Error));
        assert_eq!(diags.max_level, Some(Level::Error));

        diags.push(dummy_diag(Level::Warning));
        assert_eq!(diags.max_level, Some(Level::Error));
    }

    #[test]
    fn multibyte_column_counts_chars() {
        let source = virtual_source("<test>", "éa");
        assert_eq!(source.to_pos(2), (0, 1));
        assert_eq!(source.to_pos(3), (0, 2));
    }

    #[test]
    fn crlf_counts_carriage_return_as_column() {
        let source = virtual_source("<test>", "a\r\nb");
        assert_eq!(source.to_pos(1), (0, 1));
        assert_eq!(source.to_pos(2), (0, 2));
        assert_eq!(source.to_pos(3), (1, 0));
    }

    #[test]
    fn display_span_start() {
        let mut session = ParseSession::new();
        let id = session.add_source(virtual_source("<test>", "foo\nbar"));
        assert_eq!(
            session.display_span_start(Span::new(id, 5, 7)),
            "<test>:2:2"
        );
    }

    #[test]
    fn display_diag() {
        let mut session = ParseSession::new();
        let id = session.add_source(virtual_source("<test>", "foo\nbar"));
        let diag = Diagnostic::new(Level::Error, Span::new(id, 4, 7), "oops".into());
        assert_eq!(session.display_diag(&diag), "Error: oops at <test>:2:1");
    }

    #[test]
    fn multiple_files() {
        let mut session = ParseSession::new();
        let a = session.add_source(virtual_source("a", "one"));
        let b = session.add_source(virtual_source("b", "two"));
        assert_ne!(a, b);
        assert_eq!(session.source(a).text, "one");
        assert_eq!(session.source(b).text, "two");
        assert_eq!(session.display_span(Span::new(b, 0, 3)), "b:1:1-1:4");
    }

    #[test]
    fn diagnostics_deref() {
        let mut diags = Diagnostics::new();
        assert!(diags.is_empty());
        diags.push(dummy_diag(Level::Warning));
        diags.push(dummy_diag(Level::Error));
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[1].level, Level::Error);
    }

    #[test]
    fn from_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "foo\nbar").unwrap();
        let source = SourceFile::from_file(file.path().to_path_buf()).unwrap();
        assert_eq!(source.text, "foo\nbar");
        assert_eq!(source.line_starts, [0, 4]);
        assert!(matches!(&source.path, FilePath::Real(p) if p == file.path()));
    }

    #[test]
    fn add_source_file_display() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "foo").unwrap();
        let mut session = ParseSession::new();
        let id = session.add_source_file(file.path().to_path_buf()).unwrap();
        let path = file.path().to_str().unwrap();
        assert_eq!(
            session.display_span(Span::new(id, 0, 3)),
            format!("{path}:1:1-1:4")
        );
    }

    #[test]
    fn add_source_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let mut session = ParseSession::new();
        let err = session
            .add_source_file(dir.path().join("missing.rho"))
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }
}
