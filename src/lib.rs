pub mod backends;
pub mod data;
pub mod error;
pub mod unix;

use async_trait::async_trait;
use data::{File, FileType};

use crate::error::Result;

#[derive(Debug, Clone)]
pub struct TransitProgress {
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub processed_files: u64,
    pub total_files: u64,
}

#[derive(Debug, Clone)]
pub enum TransitState {
    Normal,
    Exists,
}

#[derive(Debug, Clone)]
pub enum TransitProgressResponse {
    ContinueOrAbort,
    Abort,
}

pub async fn move_files_between<S: AsRef<str>>(
    from_backend: impl FSBackend,
    to_backend: impl FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}

pub async fn copy_files_between<S: AsRef<str>>(
    from_backend: impl FSBackend,
    to_backend: impl FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}

pub async fn move_files_between_with_progress<
    S: AsRef<str>,
    F: FnMut(TransitProgress) -> TransitProgressResponse,
>(
    from_backend: impl FSBackend,
    to_backend: impl FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}

pub async fn copy_files_between_with_progress<
    S: AsRef<str>,
    F: FnMut(TransitProgress) -> TransitProgressResponse,
>(
    from_backend: impl FSBackend,
    to_backend: impl FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}

#[async_trait]
pub trait FSBackend {
    async fn exists(&mut self, path: &str) -> Result<bool>;
    async fn get_file_type(&mut self, path: &str) -> Result<FileType>;
    async fn retrieve_files(&mut self, paths: Vec<String>) -> Result<Vec<File>>;
    async fn retrieve_file_content(&mut self, path: &str) -> Result<Vec<u8>>;
    async fn create_file(&mut self, path: &str, contents: Option<&[u8]>) -> Result<()>;
    async fn create_dir(&mut self, path: &str) -> Result<()>;
    async fn read_dir(&mut self, path: &str) -> Result<Vec<File>>;
    async fn remove_file(&mut self, path: &str) -> Result<()>;
    async fn remove_dir(&mut self, path: &str) -> Result<()>;
    async fn trash(&mut self, path: &str) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use crate::backends::ftp::FTPBackend;
    use suppaftp::AsyncNativeTlsFtpStream;

    #[tokio::test]
    async fn ftp() {
        let mut ftp_stream = AsyncNativeTlsFtpStream::connect("ftp.dlptest.com:21")
            .await
            .expect("Failed to connect to FTP test server");

        ftp_stream
            .login("dlpuser", "rNrKYTX9g7z3RgJRmxWuGHbeu")
            .await
            .expect("Failed to login to FTP server");

        let mut backend = FTPBackend::new(ftp_stream);

        backend
            .unwrap()
            .quit()
            .await
            .expect("Failed to gracefully disconnect from FTP server");
    }
}
