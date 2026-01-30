//! Error types for the geocoding module.

use thiserror::Error;

/// Result type alias for geocoding operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during geocoding operations.
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP request failed.
    #[error("HTTP request failed: {0}")]
    HttpRequest(#[from] reqwest::Error),

    /// Failed to parse JSON response.
    #[error("Failed to parse JSON response: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Rate limit exceeded (Nominatim allows 1 request/second).
    #[error("Rate limit exceeded, please wait before retrying")]
    RateLimitExceeded,

    /// No results found for the address.
    #[error("No results found for address: {0}")]
    NoResults(String),

    /// Invalid address format.
    #[error("Invalid address format: {0}")]
    InvalidAddress(String),

    /// API returned an error.
    #[error("Nominatim API error: {0}")]
    ApiError(String),
}
