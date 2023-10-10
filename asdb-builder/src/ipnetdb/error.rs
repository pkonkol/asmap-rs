// TODO move to thiserror

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("request failed (which?)")]
    RequestError,
    #[error("problem with MMDB file")]
    DbReadError(#[from] maxminddb::MaxMindDBError),
}
// impl Display for Error {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "whois error {self:?}")
//     }
// }

// impl std::error::Error for Error {}

// impl From<maxminddb::MaxMindDBError> for Error {
//     fn from(value: maxminddb::MaxMindDBError) -> Self {
//         println!("{value}");
//         Error::DbReadError(value.to_string())
//     }
// }
