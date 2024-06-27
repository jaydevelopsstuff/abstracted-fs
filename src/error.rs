pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Rust StdIO Error (${0})")]
    StdIO(std::io::ErrorKind),
    #[error("Could not construct FileMetadata without a file name")]
    NoFileName,
    #[error("Failed to convert string to UTF-8")]
    NotUtf8,
    #[error("Operation '{0}' is unsupported on platform '{1}'")]
    Unsupported(String, String),
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::StdIO(value.kind())
    }
}
