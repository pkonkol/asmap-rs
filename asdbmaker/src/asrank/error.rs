pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("request error")]
    Request,
    #[error("json error")]
    Json(#[from] serde_json::Error),
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("generic db error")]
    Database(String),
    #[error("bulk write error")]
    BulkWrite(String),
    #[error("bulk write duplicates found")]
    BulkWriteDuplicates,
}

impl From<asdb::Error> for Error {
    fn from(e: asdb::Error) -> Self {
        match e {
            asdb::Error::DuplicatesFound(w) => Self::BulkWriteDuplicates,
            asdb::Error::BulkWrite(v) => Self::BulkWrite(v),
            _ => Self::Database(e.to_string()),
        }
    }
}
