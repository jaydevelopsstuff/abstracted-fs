use std::sync::Arc;

use crate::data::FileType;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Rust StdIO Error (${0})")]
    StdIO(std::io::ErrorKind),
    #[error("FTP Error ({0})")]
    FTP(#[from] Arc<suppaftp::types::FtpError>),
    #[error("SFTP Error ({0})")]
    SFTP(#[from] russh_sftp::client::error::Error),
    #[error("Trash Error ({0})")]
    Trash(#[from] Arc<trash::Error>),

    #[error("Cannot copy or move file of type")] // TODO
    CannotCopyOrMoveFileType(FileType),
    #[error("File at path '{0}' is nonexistent")]
    FileNonexistent(String),
    #[error("File '{0}' already exists")]
    FileAlreadyExists(String),
    #[error("Could not construct FileMetadata without a file name")]
    NoFileName,
    #[error("Failed to convert string to UTF-8")]
    NotUtf8,
    #[error("Operation '{0}' is unsupported on platform '{1}'")]
    Unsupported(String, String),
}

impl Error {
    pub fn is_already_exists_error(&self) -> bool {
        matches!(
            self,
            Self::FileAlreadyExists(_) | Self::StdIO(std::io::ErrorKind::AlreadyExists)
        )
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::StdIO(value.kind())
    }
}
