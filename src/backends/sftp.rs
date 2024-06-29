use std::path::Path;

use async_trait::async_trait;
use russh_sftp::client::fs::Metadata as SFTPMetadata;
use russh_sftp::client::SftpSession;
use tokio::io::AsyncWriteExt;

use crate::data::{File, FileType, Metadata};
use crate::error::{Error, Result};
use crate::FSBackend;

pub struct SFTPBackend {
    pub session: SftpSession,
}

impl SFTPBackend {
    pub fn new(session: SftpSession) -> Self {
        Self { session }
    }

    pub fn inner(&mut self) -> &mut SftpSession {
        &mut self.session
    }

    pub fn unwrap(self) -> SftpSession {
        self.session
    }
}

#[async_trait]
impl FSBackend for SFTPBackend {
    async fn exists(&self, path: &str) -> Result<bool> {
        Ok(self.session.try_exists(path).await?)
    }

    async fn get_file_type(&self, path: &str) -> Result<FileType> {
        Ok(file_type_from_sftp_metadata(
            &self.session.metadata(path).await?,
        ))
    }

    async fn retrieve_files(&self, paths: Vec<String>) -> Result<Vec<File>> {
        let mut files = vec![];

        for path in paths {
            let path_std = Path::new(&path);

            files.push(File {
                path: path.clone(),
                name: path_std
                    .file_name()
                    .ok_or(Error::NoFileName)?
                    .to_str()
                    .unwrap() // Input paths are already Unicode
                    .to_string(),
                extension: path_std
                    .extension()
                    .and_then(|os_str| Some(os_str.to_str().unwrap().to_string())), // Input paths are already Unicode
                metadata: self.session.metadata(path).await?.into(),
            })
        }

        Ok(files)
    }

    async fn retrieve_file_content(&self, path: &str) -> Result<Vec<u8>> {
        Ok(self.session.read(path).await?)
    }

    async fn create_file(&self, path: &str, contents: Option<&[u8]>) -> Result<()> {
        if self.exists(&path).await? {
            return Err(Error::AlreadyExists(path.to_string()));
        }

        let mut file = self.session.create(path).await?;

        if let Some(contents) = contents {
            file.write_all(contents).await?;
        }

        Ok(())
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        self.session.create_dir(path).await?;
        Ok(())
    }

    async fn read_dir(&self, path: &str) -> Result<Vec<File>> {
        Ok(self
            .session
            .read_dir(path)
            .await?
            .map(|file| {
                let path = format!("{path}/{}", file.file_name());
                let extension = Path::new(&path)
                    .extension()
                    .and_then(|os_str| os_str.to_str().and_then(|str| Some(str.to_string())));

                File {
                    path,
                    name: file.file_name(),
                    extension,
                    metadata: file.metadata().into(),
                }
            })
            .collect())
    }

    async fn remove_file(&self, path: &str) -> Result<()> {
        self.session.remove_file(path).await?;
        Ok(())
    }

    async fn remove_dir(&self, path: &str) -> Result<()> {
        self.session.remove_dir(path).await?;
        Ok(())
    }

    async fn trash(&self, _paths: &[&str]) -> Result<()> {
        return Err(Error::Unsupported("trash".into(), "Unsupported".into()));
    }
}

impl From<SFTPMetadata> for Metadata {
    fn from(sftp_metadata: SFTPMetadata) -> Self {
        Metadata {
            r#type: file_type_from_sftp_metadata(&sftp_metadata),
            modified: sftp_metadata.modified().ok(),
            accessed: sftp_metadata.accessed().ok(),
            created: None,
            size: sftp_metadata.size,
            readonly: false, // FIXME: Assumption
            unix_file_permissions: sftp_metadata
                .permissions
                .and_then(|permission_bits| Some(permission_bits.into())),
        }
    }
}

pub fn file_type_from_sftp_metadata(sftp_metadata: &SFTPMetadata) -> FileType {
    // FIXME: Should we extract all these file_type() calls into a variable?
    FileType::from_complex_bools((
        sftp_metadata.is_regular(),
        sftp_metadata.is_dir(),
        sftp_metadata.is_symlink(),
        false,
        sftp_metadata.is_fifo(),
        sftp_metadata.is_character(),
        sftp_metadata.is_block(),
    ))
}
