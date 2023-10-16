pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Could not download file")]
    DownloadError,
    #[error("something with reading csv file")]
    CsvError(#[from] csv::Error),
    #[error("io")]
    IoError(#[from] std::io::Error),
}
