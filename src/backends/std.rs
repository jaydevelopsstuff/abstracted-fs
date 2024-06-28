use std::fs::Metadata as StdMetadata;
use std::path::Path;

use tokio::fs;

use crate::data::{File, FileType, Metadata};
use crate::error::{Error, Result};
use crate::FSBackend;

pub struct StdBackend;

impl FSBackend for StdBackend {
    async fn exists<P: AsRef<Path>>(&mut self, path: P) -> Result<bool> {
        Ok(path.as_ref().exists())
    }

    async fn get_file_type<P: AsRef<Path>>(&mut self, path: P) -> Result<FileType> {
        Ok(file_type_from_std_metadata(
            &tokio::fs::metadata(path).await?,
        ))
    }

    async fn retrieve_files<P: AsRef<Path>>(&mut self, paths: Vec<P>) -> Result<Vec<File>> {
        let mut files = vec![];

        for path in paths {
            let path = path.as_ref();

            files.push(File {
                path: path.to_str().ok_or(Error::NotUtf8)?.to_string(),
                name: path
                    .file_name()
                    .ok_or(Error::NoFileName)?
                    .to_str()
                    .ok_or(Error::NotUtf8)?
                    .to_string(),
                extension: path
                    .extension()
                    .and_then(|os_str| os_str.to_str().and_then(|str| Some(str.to_string()))),
                metadata: tokio::fs::metadata(path).await?.into(),
            });
        }

        Ok(files)
    }

    async fn retrieve_file_content<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>> {
        Ok(tokio::fs::read(path).await?)
    }

    async fn create_file<P: AsRef<Path>>(
        &mut self,
        path: P,
        contents: Option<&[u8]>,
    ) -> Result<()> {
        tokio::fs::File::create_new(&path).await?;
        if let Some(contents) = contents {
            tokio::fs::write(&path, contents).await?;
        }
        Ok(())
    }

    async fn create_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        tokio::fs::create_dir(path).await?;
        Ok(())
    }

    async fn read_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<File>> {
        let mut files = vec![];
        let mut result = fs::read_dir(path).await?;

        // `let Ok()` implies this loop will end whenever an error is encountered (and not return it)
        while let Ok(Some(entry)) = result.next_entry().await {
            files.push(File {
                path: entry.path().to_str().ok_or(Error::NotUtf8)?.to_string(),
                name: entry
                    .file_name()
                    .to_str()
                    .ok_or(Error::NotUtf8)?
                    .to_string(),
                extension: entry
                    .path()
                    .extension()
                    .and_then(|os_str| os_str.to_str().and_then(|str| Some(str.to_string()))),
                metadata: entry.metadata().await?.into(),
            });
        }

        Ok(files)
    }

    async fn remove_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        tokio::fs::remove_file(path).await?;
        Ok(())
    }

    async fn remove_dir<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        tokio::fs::remove_dir(path).await?;
        Ok(())
    }

    async fn trash<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        trash::delete(path)?; // FIXME: This isn't async...
        Ok(())
    }
}

impl From<StdMetadata> for Metadata {
    fn from(std_metadata: StdMetadata) -> Self {
        #[cfg(unix)]
        use std::os::unix::fs::MetadataExt;
        #[cfg(target_os = "windows")]
        use std::os::windows::fs::MetadataExt;

        // This feels really ugly, is there a better way to conditionally compile this?
        #[cfg(unix)]
        let size = Some(std_metadata.size());
        #[cfg(windows)]
        let size = Some(std_metadata.file_size());
        #[cfg(all(not(unix), not(windows)))]
        let size = None;

        Metadata {
            r#type: file_type_from_std_metadata(&std_metadata),
            size,
            modified: std_metadata.modified().ok(),
            accessed: std_metadata.accessed().ok(),
            created: std_metadata.created().ok(),
            readonly: std_metadata.permissions().readonly(),
            unix_file_permissions: if cfg!(unix) {
                Some(std_metadata.mode().into())
            } else {
                None
            },
        }
    }
}

fn file_type_from_std_metadata(std_metadata: &StdMetadata) -> FileType {
    #[cfg(unix)]
    use std::os::unix::fs::FileTypeExt;

    if cfg!(unix) {
        // FIXME: Should we extract all these file_type() calls into a variable?
        FileType::from_complex_bools((
            std_metadata.is_file(),
            std_metadata.is_dir(),
            std_metadata.is_symlink(),
            std_metadata.file_type().is_socket(),
            std_metadata.file_type().is_fifo(),
            std_metadata.file_type().is_char_device(),
            std_metadata.file_type().is_block_device(),
        ))
    } else {
        FileType::from_bools(
            std_metadata.is_file(),
            std_metadata.is_dir(),
            std_metadata.is_symlink(),
        )
    }
}
