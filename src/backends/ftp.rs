use std::path::Path;
use std::str::FromStr;

use async_trait::async_trait;
use suppaftp::AsyncNativeTlsFtpStream;

use crate::data::{File, FileType, Metadata};
use crate::error::{Error, Result};
use crate::FSBackend;

pub struct FTPBackend {
    pub stream: AsyncNativeTlsFtpStream,
}

impl FTPBackend {
    pub fn new(stream: AsyncNativeTlsFtpStream) -> Self {
        Self { stream }
    }

    pub fn inner(&mut self) -> &mut AsyncNativeTlsFtpStream {
        &mut self.stream
    }

    pub fn unwrap(self) -> AsyncNativeTlsFtpStream {
        self.stream
    }
}

#[async_trait]
impl FSBackend for FTPBackend {
    async fn exists(&mut self, path: &str) -> Result<bool> {
        todo!()
    }

    async fn get_file_type(&mut self, path: &str) -> Result<FileType> {
        todo!()
    }

    async fn retrieve_files(&mut self, paths: Vec<String>) -> Result<Vec<File>> {
        let mut files = vec![];

        for path in paths {
            files.push(self.stream.mlst(Some(&path)).await?)
        }

        println!("{:?}", files);

        Ok(vec![])
    }

    async fn retrieve_file_content(&mut self, path: &str) -> Result<Vec<u8>> {
        todo!()
    }

    async fn create_file(&mut self, path: &str, contents: Option<&[u8]>) -> Result<()> {
        self.stream
            .put_file(path, &mut contents.unwrap_or(&[]))
            .await?;

        Ok(())
    }

    async fn create_dir(&mut self, path: &str) -> Result<()> {
        self.stream.mkdir(path).await?;
        Ok(())
    }

    async fn read_dir(&mut self, path: &str) -> Result<Vec<File>> {
        Ok(self
            .stream
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

    async fn remove_file(&mut self, path: &str) -> Result<()> {
        self.stream.rm(path).await?;
        Ok(())
    }

    async fn remove_dir(&mut self, path: &str) -> Result<()> {
        self.stream.rmdir(path).await?;
        Ok(())
    }

    async fn trash(&mut self, _paths: &[&str]) -> Result<()> {
        Err(Error::Unsupported("trash".into(), "FTP".into()))
    }
}
