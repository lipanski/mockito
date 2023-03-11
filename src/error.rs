use std::error::Error as ErrorTrait;
use std::fmt::Display;

///
/// Contains information about an error occurence
///
#[derive(Debug)]
pub struct Error {
    /// The type of this error
    pub kind: ErrorKind,
    /// Some errors come with more context
    pub context: Option<String>,
}

impl Error {
    pub(crate) fn new(kind: ErrorKind) -> Error {
        Error {
            kind,
            context: None,
        }
    }

    pub(crate) fn new_with_context(kind: ErrorKind, context: impl Display) -> Error {
        Error {
            kind,
            context: Some(context.to_string()),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (context: {})",
            self.kind.description(),
            self.context.as_ref().unwrap_or(&"none".to_string())
        )
    }
}

impl ErrorTrait for Error {}

///
/// The type of an error
///
#[derive(Debug)]
pub enum ErrorKind {
    /// The server is not running
    ServerFailure,
    /// The server is busy
    ServerBusy,
    /// A lock can't be bypassed
    Deadlock,
    /// Could not deliver a response
    ResponseFailure,
    /// The status code is invalid or out of range
    InvalidStatusCode,
    /// Failed to read the request body
    RequestBodyFailure,
    /// Failed to write the response body
    ResponseBodyFailure,
    /// File not found
    FileNotFound,
}

impl ErrorKind {
    fn description(&self) -> &'static str {
        match self {
            ErrorKind::ServerFailure => "the server is not running",
            ErrorKind::ServerBusy => "the server is busy",
            ErrorKind::Deadlock => "a lock can't be bypassed",
            ErrorKind::ResponseFailure => "could not deliver a response",
            ErrorKind::InvalidStatusCode => "invalid status code",
            ErrorKind::RequestBodyFailure => "failed to read the request body",
            ErrorKind::ResponseBodyFailure => "failed to write the response body",
            ErrorKind::FileNotFound => "file not found",
        }
    }
}
