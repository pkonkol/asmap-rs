use std::fmt::Display;

use crate::asrank;

#[derive(Debug)]
pub enum Error {
    ImportError(String),
    DatabaseError(String),
    JsonError(String),
    IoError(String),
    InitError,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "asdbmaker error {self:?}")
    }
}

impl std::error::Error for Error {}

impl From<asrank::Error> for Error {
    fn from(e: asrank::Error) -> Self {
        Self::ImportError(e.to_string())
    }
}

impl From<asdb::Error> for Error {
    fn from(e: asdb::Error) -> Self {
        Self::DatabaseError(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::JsonError(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
