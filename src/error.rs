pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Rust StdIO Error (${0})")]
    StdIO(std::io::ErrorKind),
    #[error("FTP Error ({0})")]
    FTP(#[from] suppaftp::types::FtpError),
    #[error("SFTP Error ({0})")]
    SFTP(#[from] russh_sftp::client::error::Error),
    #[error("Trash Error ({0})")]
    Trash(#[from] trash::Error),

    #[error("File '{0}' already exists")]
    AlreadyExists(String),
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
