use std::path::Path;

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
}

impl FSBackend for SFTPBackend {
    async fn exists<P: AsRef<Path>>(&mut self, path: P) -> Result<bool> {
        Ok(self
            .session
            .try_exists(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
            .await?)
    }

    async fn get_file_type<P: AsRef<Path>>(&mut self, path: P) -> Result<FileType> {
        Ok(file_type_from_sftp_metadata(
            &self
                .session
                .metadata(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
                .await?,
        ))
    }

    async fn retrieve_files<P: AsRef<Path>>(&mut self, paths: Vec<P>) -> Result<Vec<File>> {
        let mut files = vec![];

        for path in paths {
            files.push(File {
                path: path.as_ref().to_str().ok_or(Error::NotUtf8)?.to_string(),
                name: path
                    .as_ref()
                    .file_name()
                    .ok_or(Error::NoFileName)?
                    .to_str()
                    .ok_or(Error::NotUtf8)?
                    .to_string(),
                extension: path
                    .as_ref()
                    .extension()
                    .and_then(|os_str| os_str.to_str().and_then(|str| Some(str.to_string()))),
                metadata: self
                    .session
                    .metadata(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
                    .await?
                    .into(),
            })
        }

        Ok(files)
    }

    async fn retrieve_file_content<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>> {
        Ok(self
            .session
            .read(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
            .await?)
    }

    async fn create_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        contents: Option<&[u8]>,
    ) -> Result<()> {
        if self.exists(&path).await? {
            return Err(Error::AlreadyExists(
                path.as_ref().to_str().ok_or(Error::NotUtf8)?.to_string(),
            ));
        }

        let mut file = self
            .session
            .create(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
            .await?;

        if let Some(contents) = contents {
            file.write_all(contents).await?;
        }

        Ok(())
    }

    async fn create_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.session
            .create_dir(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
            .await?;
        Ok(())
    }

    async fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<File>> {
        let path_str = path.as_ref().to_str().ok_or(Error::NotUtf8)?;

        Ok(self
            .session
            .read_dir(path_str)
            .await?
            .map(|file| {
                let path = format!("{path_str}/{}", file.file_name());
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

    async fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.session
            .remove_file(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
            .await?;
        Ok(())
    }

    async fn remove_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.session
            .remove_dir(path.as_ref().to_str().ok_or(Error::NotUtf8)?)
            .await?;
        Ok(())
    }

    async fn trash<P: AsRef<Path>>(&mut self, _path: P) -> Result<()> {
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
