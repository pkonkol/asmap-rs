const DUPLICATES_CODE_ERROR: i32 = 11000;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("connection error")]
    Connection(String),
    #[error("bulk write error other than duplicates")]
    BulkWrite(String),
    #[error("duplicates found while inserting, count: {0}")]
    DuplicatesFound(u64),
    #[error("as not found")]
    AsNotFound,
}

impl From<mongodb::error::Error> for Error {
    fn from(value: mongodb::error::Error) -> Self {
        match value.kind.as_ref() { 
            mongodb::error::ErrorKind::BulkWrite(e) => { 
                let v = e.write_errors.as_deref().unwrap();
                let mut count = 0;
                for x in v {
                    if x.code != DUPLICATES_CODE_ERROR {
                        return Self::BulkWrite(format!("{e:?}"));
                    }
                    count += 1;
                }
                Self::DuplicatesFound(count)
            }
            _ => {
                Self::Connection(value.to_string())
            }
        }
    }
}
