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
    async fn retrieve_files(&self, paths: &[&str]) -> Result<Vec<File>>;
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

    async fn calculate_total_size(&self, paths: &[&str]) -> Result<u64> {
        let mut total_size = 0;

        let mut dirs_to_process = vec![];

        for file in self.retrieve_files(paths).await? {
            if file.metadata.r#type == FileType::Dir {
                dirs_to_process.push(file.path);
            } else {
                total_size += file.metadata.size.unwrap_or(0);
            }
        }

        while !dirs_to_process.is_empty() {
            let mut new_dirs_to_process = vec![];

            for dir in dirs_to_process {
                for file in self.read_dir(&dir).await? {
                    if file.metadata.r#type == FileType::Dir {
                        new_dirs_to_process.push(file.path);
                    } else {
                        total_size += file.metadata.size.unwrap_or(0);
                    }
                }
            }

            dirs_to_process = new_dirs_to_process;
        }

        Ok(total_size)
    }

    async fn remove_all(&self, paths: &[&str]) -> Result<()> {
        let mut dirs_to_process = vec![];

        for path in paths {
            if self.get_file_type(path).await? == FileType::Dir {
                dirs_to_process.push(path.to_string());
            } else {
                self.remove_file(path).await?;
            }
        }

        let mut dirs_to_remove = vec![];

        while !dirs_to_process.is_empty() {
            let mut new_dirs_to_process = vec![];

            for dir in dirs_to_process {
                let results = self.read_dir(&dir).await?;

                for file in results {
                    if file.metadata.r#type == FileType::Dir {
                        new_dirs_to_process.push(file.path);
                    } else {
                        self.remove_file(&file.path).await?;
                    }
                }

                dirs_to_remove.push(dir);
            }

            dirs_to_process = new_dirs_to_process;
        }

        while let Some(dir) = dirs_to_remove.pop() {
            self.remove_dir(&dir).await?;
        }

        Ok(())
    }
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
