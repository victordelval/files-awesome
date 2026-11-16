use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, CoreError>;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("could not {op} {}: {source}", path.display())]
    Io {
        op: &'static str,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{} is not valid UTF-8 text (first invalid byte at offset {offset})", path.display())]
    NotUtf8 { path: PathBuf, offset: usize },

    #[error("{} is {size} bytes; the limit is {limit} bytes", path.display())]
    TooLarge {
        path: PathBuf,
        size: u64,
        limit: u64,
    },

    #[error("{} is not a regular file", path.display())]
    NotAFile { path: PathBuf },

    #[error("{} was modified by another program since it was opened", path.display())]
    Conflict { path: PathBuf },

    #[error("the document has not been saved to a file yet")]
    NoPath,
}

impl CoreError {
    pub(crate) fn io(op: &'static str, path: &Path, source: std::io::Error) -> Self {
        Self::Io {
            op,
            path: path.to_path_buf(),
            source,
        }
    }

    /// Stable, machine-readable identifier for front ends.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Io { source, .. } => match source.kind() {
                std::io::ErrorKind::NotFound => "notFound",
                std::io::ErrorKind::PermissionDenied => "permissionDenied",
                _ => "io",
            },
            Self::NotUtf8 { .. } => "notUtf8",
            Self::TooLarge { .. } => "tooLarge",
            Self::NotAFile { .. } => "notAFile",
            Self::Conflict { .. } => "conflict",
            Self::NoPath => "noPath",
        }
    }
}
