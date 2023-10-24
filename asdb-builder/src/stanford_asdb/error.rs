pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not download file")]
    Download,
    #[error("something with reading csv file")]
    Csv(#[from] csv::Error),
    #[error("io")]
    Io(#[from] std::io::Error),
}
