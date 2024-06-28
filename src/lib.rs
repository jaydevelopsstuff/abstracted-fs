pub mod backends;
pub mod data;
pub mod error;
pub mod unix;

use std::path::Path;

use data::{File, FileType};

use crate::error::Result;

pub trait FSBackend {
    async fn exists<P: AsRef<Path>>(&mut self, path: P) -> Result<bool>;
    async fn get_file_type<P: AsRef<Path>>(&mut self, path: P) -> Result<FileType>;
    async fn retrieve_files<P: AsRef<Path>>(&mut self, paths: Vec<P>) -> Result<Vec<File>>;
    async fn retrieve_file_content<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>>;
    async fn create_file<P: AsRef<Path>>(&mut self, path: P, contents: Option<&[u8]>)
        -> Result<()>;
    async fn create_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()>;
    async fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<File>>;
    async fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()>;
    async fn remove_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()>;
    async fn trash<P: AsRef<Path>>(&mut self, path: P) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use crate::{backends::ftp::FTPBackend, FSBackend};
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

        let read_results = backend.read_dir("/").await.expect("Failed to read dir");

        backend
            .unwrap()
            .quit()
            .await
            .expect("Failed to gracefully disconnect from FTP server");
    }
}
