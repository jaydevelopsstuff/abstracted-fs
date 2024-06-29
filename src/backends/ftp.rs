use std::path::Path;
use std::str::FromStr;

use async_trait::async_trait;
use suppaftp::AsyncNativeTlsFtpStream;
use tokio::sync::Mutex;

use crate::data::{File, FileType, Metadata};
use crate::error::{Error, Result};
use crate::FSBackend;

pub struct FTPBackend {
    pub stream: Mutex<AsyncNativeTlsFtpStream>,
}

impl FTPBackend {
    pub fn new(stream: AsyncNativeTlsFtpStream) -> Self {
        Self {
            stream: Mutex::new(stream),
        }
    }

    pub fn inner(&mut self) -> &mut AsyncNativeTlsFtpStream {
        self.stream.get_mut()
    }

    pub fn unwrap(self) -> AsyncNativeTlsFtpStream {
        self.stream.into_inner()
    }
}

#[async_trait]
impl FSBackend for FTPBackend {
    async fn exists(&self, path: &str) -> Result<bool> {
        todo!()
    }

    async fn get_file_type(&self, path: &str) -> Result<FileType> {
        todo!()
    }

    async fn retrieve_files(&self, paths: Vec<String>) -> Result<Vec<File>> {
        let mut stream = self.stream.lock().await;
        let mut files = vec![];

        for path in paths {
            files.push(stream.mlst(Some(&path)).await?)
        }

        println!("{:?}", files);

        Ok(vec![])
    }

    async fn retrieve_file_content(&self, path: &str) -> Result<Vec<u8>> {
        todo!()
    }

    async fn create_file(&self, path: &str, contents: Option<&[u8]>) -> Result<()> {
        self.stream
            .lock()
            .await
            .put_file(path, &mut contents.unwrap_or(&[]))
            .await?;

        Ok(())
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        self.stream.lock().await.mkdir(path).await?;
        Ok(())
    }

    async fn read_dir(&self, path: &str) -> Result<Vec<File>> {
        Ok(self
            .stream
            .lock()
            .await
            .list(Some(path))
            .await?
            .into_iter()
            .filter_map(|file_str| suppaftp::list::File::from_str(&file_str).ok())
            .map(|file| {
                let path = format!("{path}/{}", file.name());
                let extension = Path::new(&path)
                    .extension()
                    .and_then(|os_str| os_str.to_str().and_then(|str| Some(str.to_string())));

                File {
                    path,
                    name: file.name().to_string(),
                    extension,
                    metadata: Metadata {
                        r#type: FileType::from_bools(
                            file.is_file(),
                            file.is_directory(),
                            file.is_symlink(),
                        ),
                        modified: Some(file.modified()),
                        accessed: None,
                        created: None,
                        size: Some(file.size() as u64),
                        readonly: false, // FIXME: Assumption
                        unix_file_permissions: None,
                    },
                }
            })
            .collect())
    }

    async fn remove_file(&self, path: &str) -> Result<()> {
        self.stream.lock().await.rm(path).await?;
        Ok(())
    }

    async fn remove_dir(&self, path: &str) -> Result<()> {
        self.stream.lock().await.rmdir(path).await?;
        Ok(())
    }

    async fn trash(&self, _paths: &[&str]) -> Result<()> {
        Err(Error::Unsupported("trash".into(), "FTP".into()))
    }
}
