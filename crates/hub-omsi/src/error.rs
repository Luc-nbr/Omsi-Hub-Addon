//! Why a file could not be read.

use std::path::PathBuf;

#[derive(Debug)]
pub enum ReadError {
    /// The file is not there, or could not be read.
    Io { path: PathBuf, error: std::io::Error },
    /// It is there but not in the form expected: what, and at which line (1 = the first).
    Format { path: PathBuf, line: usize, what: String },
    /// No such file in the content folder nor in the OMSI 2 folder.
    NotFound(String),
}

impl ReadError {
    pub fn io(path: impl Into<PathBuf>, error: std::io::Error) -> ReadError {
        ReadError::Io { path: path.into(), error }
    }
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadError::Io { path, error } => write!(f, "{}: {error}", path.display()),
            ReadError::Format { path, line, what } => write!(f, "{}:{line}: {what}", path.display()),
            ReadError::NotFound(rel) => write!(f, "{rel} was not found"),
        }
    }
}

impl std::error::Error for ReadError {}
