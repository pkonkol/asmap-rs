//! Error types for WHOIS API operations.

use std::fmt::Display;

/// Errors that can occur during WHOIS API operations.
#[derive(Debug)]
pub enum Error {
    /// HTTP request failed
    Request(reqwest::Error),
    /// Failed to parse API response
    Parse(String),
    /// Object not found in WHOIS database
    NotFound(String),
    /// Rate limit exceeded
    RateLimited,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Request(e) => write!(f, "WHOIS request error: {e}"),
            Error::Parse(msg) => write!(f, "WHOIS parse error: {msg}"),
            Error::NotFound(obj) => write!(f, "WHOIS object not found: {obj}"),
            Error::RateLimited => write!(f, "WHOIS API rate limit exceeded"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Request(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Request(e)
    }
}

/// Result type for WHOIS operations.
pub type Result<T> = std::result::Result<T, Error>;
