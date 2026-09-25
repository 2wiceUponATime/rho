use std::{cell::RefCell, fs, io, path::PathBuf};

use crate::{interner::Interner, span::Span};

#[derive(Clone, Copy, Debug)]
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
        let idx = self.line_start_idx(offset);
        let line_start = self.line_starts[idx as usize];
        (idx, offset - line_start)
    }

    fn line_start_idx(&self, offset: u32) -> u32 {
        match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx as u32,
            Err(next) => (next - 1) as u32,
        }
    }
}

#[derive(Debug)]
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
pub struct ParseSession {
    source_files: Vec<SourceFile>,
    pub interner: RefCell<Interner>,
    pub diagnostics: RefCell<Vec<Diagnostic>>,
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

    pub fn diag(&self, diag: Diagnostic) {
        self.diagnostics.borrow_mut().push(diag);
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
