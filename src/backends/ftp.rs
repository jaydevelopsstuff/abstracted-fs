use std::path::Path;
use std::str::FromStr;

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

    pub fn unwrap(self) -> AsyncNativeTlsFtpStream {
        self.stream
    }
}

impl FSBackend for FTPBackend {
    async fn exists<P: AsRef<Path>>(&mut self, path: P) -> Result<bool> {
        todo!()
    }

    async fn get_file_type<P: AsRef<Path>>(&mut self, path: P) -> Result<FileType> {
        todo!()
    }

    async fn retrieve_files<P: AsRef<Path>>(&mut self, paths: Vec<P>) -> Result<Vec<File>> {
        let mut files = vec![];

        for path in paths {
            files.push(
                self.stream
                    .mlst(Some(path.as_ref().to_str().ok_or(Error::NotUtf8)?))
                    .await?,
            )
        }

        println!("{:?}", files);

        Ok(vec![])
    }

    async fn retrieve_file_content<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>> {
        todo!()
    }

    async fn create_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        contents: Option<&[u8]>,
    ) -> Result<()> {
        self.stream
            .put_file(
                path.as_ref().to_str().ok_or(Error::NotUtf8)?,
                &mut contents.unwrap_or(&[]),
            )
            .await?;

        Ok(())
    }

    async fn create_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.stream
            .mkdir(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
            .await?;
        Ok(())
    }

    async fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<File>> {
        let path_str = path.as_ref().to_str().ok_or(Error::NotUtf8)?;
        Ok(self
            .stream
            .list(Some(path_str))
            .await?
            .into_iter()
            .filter_map(|file_str| suppaftp::list::File::from_str(&file_str).ok())
            .map(|file| {
                let path = format!("{path_str}/{}", file.name());
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

    async fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.stream
            .rm(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
            .await?;
        Ok(())
    }

    async fn remove_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.stream
            .rmdir(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
            .await?;
        Ok(())
    }

    async fn trash<P: AsRef<Path>>(&mut self, _path: P) -> Result<()> {
        Err(Error::Unsupported("trash".into(), "FTP".into()))
    }
}
