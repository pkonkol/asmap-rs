use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    ImportError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "asdbmaker error {self:?}")
    }
}

impl std::error::Error for Error {}

// impl From<tokio::task::JoinError> for Error {
//     fn from(e: tokio::task::JoinError) -> Self  {
//         Error::FailedSpawnBlocking
//     }
// }

pub type Result<T> = std::result::Result<T, Error>;
