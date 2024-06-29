pub mod backends;
pub mod data;
pub mod error;
pub mod unix;

use async_trait::async_trait;
use data::{File, FileType};

use crate::error::Result;

pub async fn remove_all<B: AsMut<dyn FSBackend>, S: AsRef<str>>(
    mut backend: B,
    paths: &[S],
) -> Result<()> {
    let backend = backend.as_mut();
    let mut dirs_to_process = vec![];

    for path in paths {
        if backend.get_file_type(path.as_ref()).await? == FileType::Dir {
            dirs_to_process.push(path.as_ref().to_string());
        } else {
            backend.remove_file(path.as_ref()).await?;
        }
    }

    while !dirs_to_process.is_empty() {
        let mut new_dirs_to_process = vec![];

        for dir in dirs_to_process {
            let results = backend.read_dir(&dir).await?;
            let results_len = results.len();

            let mut files_removed = 0;
            for file in results {
                if file.metadata.r#type == FileType::Dir {
                    new_dirs_to_process.push(file.path);
                } else {
                    backend.remove_file(&file.path).await?;
                    files_removed += 1;
                }
            }

            // If this directory has been emptied (or was empty to begin with), then delete it
            if files_removed == results_len {
                backend.remove_dir(&dir).await?;
            }
        }

        dirs_to_process = new_dirs_to_process;
    }

    Ok(())
}

pub async fn move_files_between<S: AsRef<str>>(
    from_backend: &mut dyn FSBackend,
    to_backend: &mut dyn FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}

pub async fn copy_files_between<S: AsRef<str>>(
    from_backend: &mut dyn FSBackend,
    to_backend: &mut dyn FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}

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

pub async fn move_files_between_with_progress<
    S: AsRef<str>,
    F: FnMut(TransitProgress) -> TransitProgressResponse,
>(
    from_backend: &mut dyn FSBackend,
    to_backend: &mut dyn FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}

pub async fn copy_files_between_with_progress<
    S: AsRef<str>,
    F: FnMut(TransitProgress) -> TransitProgressResponse,
>(
    from_backend: &mut dyn FSBackend,
    to_backend: &mut dyn FSBackend,
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
    async fn trash(&mut self, paths: &[&str]) -> Result<()>;
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
