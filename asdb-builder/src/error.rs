use crate::{asrank, ipnetdb};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("import error")]
    Import(#[from] asrank::Error),
    #[error("database error")]
    Database(String),
    #[error("duplicate writes, count: {0}")]
    DuplicateWrites(u64),
    #[error("json error")]
    Json(#[from] serde_json::Error),
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("init error")]
    Init,
    #[error("ipnetdb error")]
    IpnetDB(#[from] ipnetdb::Error)
}

impl From<asdb::Error> for Error {
    fn from(e: asdb::Error) -> Self {
        match e {
            asdb::Error::DuplicatesFound(c) => Self::DuplicateWrites(c),
            _ => Self::Database(e.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
