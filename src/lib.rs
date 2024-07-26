pub mod backends;
pub mod data;
pub mod error;
pub mod ops;
mod util;

use async_trait::async_trait;
use data::{File, FileType};

use crate::error::Result;

#[async_trait]
pub trait FSBackend: Send + Sync {
    async fn disconnect(&self) -> Result<()>;
    async fn exists(&self, path: &str) -> Result<bool>;
    async fn get_file_type(&self, path: &str) -> Result<FileType>;
    async fn retrieve_files(&self, paths: Vec<String>) -> Result<Vec<File>>;
    async fn retrieve_file_content(&self, path: &str) -> Result<Vec<u8>>;
    async fn read_dir(&self, path: &str) -> Result<Vec<File>>;
    async fn create_file(&self, path: &str, overwrite: bool, contents: Option<&[u8]>)
        -> Result<()>;
    async fn create_dir(&self, path: &str) -> Result<()>;
    async fn rename_file(&self, path: &str, new_name: &str, overwrite: bool) -> Result<()>;
    async fn move_file(&self, from: &str, to: &str, overwrite: bool) -> Result<()>;
    async fn copy_file(&self, from: &str, to: &str, overwrite: bool) -> Result<()>;
    async fn remove_file(&self, path: &str) -> Result<()>;
    async fn remove_dir(&self, path: &str) -> Result<()>;
    async fn trash(&self, paths: &[&str]) -> Result<()>;
    async fn set_file_permissions_unix(&self, path: &str, mode: u32) -> Result<()>;
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
