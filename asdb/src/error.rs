use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    ConnectionError(String),
    AsNotFound,
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "asdb error {self:?}")
    }
}

impl std::error::Error for Error {}

impl From<mongodb::error::Error> for Error {
    fn from(value: mongodb::error::Error) -> Self {
        Self::ConnectionError(value.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
