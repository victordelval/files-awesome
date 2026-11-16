use fa_core::CoreError;
use serde::Serialize;

/// Error shape returned to the front end: `{ kind, message }`.
#[derive(Debug, Serialize)]
pub struct CmdError {
    kind: &'static str,
    message: String,
}

impl CmdError {
    pub fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn unknown_doc(id: u32) -> Self {
        Self::new("unknownDocument", format!("no open document with id {id}"))
    }

    pub fn internal(message: impl std::fmt::Display) -> Self {
        Self::new("internal", message.to_string())
    }
}

impl From<CoreError> for CmdError {
    fn from(e: CoreError) -> Self {
        Self::new(e.kind(), e.to_string())
    }
}

pub type CmdResult<T> = Result<T, CmdError>;
