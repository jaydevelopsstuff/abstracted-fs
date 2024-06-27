pub mod backends;
pub mod error;

use std::{path::Path, time::SystemTime};

use crate::error::Result;

pub trait FSBackend {
    async fn is_file<P: AsRef<Path>>(&self, path: P) -> Result<bool>;
    async fn is_dir<P: AsRef<Path>>(&self, path: P) -> Result<bool>;
    async fn is_symlink<P: AsRef<Path>>(&self, path: P) -> Result<bool>;
    async fn get_metadata<P: AsRef<Path>>(&self, paths: Vec<P>) -> Result<Vec<File>>;
    async fn get_file_content<P: AsRef<Path>>(&self, path: P) -> Result<Vec<u8>>;
    async fn create_file<P: AsRef<Path>>(&self, path: P, contents: Option<&[u8]>) -> Result<()>;
    async fn create_dir<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    async fn read_dir<P: AsRef<Path>>(&self, path: P) -> Result<Vec<File>>;
    async fn remove_file<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    async fn remove_dir<P: AsRef<Path>>(&self, path: P) -> Result<()>;
    async fn trash<P: AsRef<Path>>(&self, path: P) -> Result<()>;
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct File {
    pub path: String,
    pub name: String,
    pub extension: Option<String>,
    pub metadata: Metadata,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Metadata {
    pub r#type: FileType,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub created: Option<SystemTime>,
    pub size: Option<u64>,
    pub readonly: bool,
    pub unix_mode: Option<u32>,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum FileType {
    File,
    Dir,
    Symlink,
    Socket,
    Fifo,
    CharDevice,
    BlockDevice,
    Unknown,
}

impl FileType {
    fn from_bools(is_file: bool, is_dir: bool, is_symlink: bool) -> Self {
        Self::from_complex_bools((is_file, is_dir, is_symlink, false, false, false, false))
    }

    fn from_complex_bools(bools: (bool, bool, bool, bool, bool, bool, bool)) -> Self {
        match bools {
            (true, false, false, false, false, false, false) => Self::File,
            (false, true, false, false, false, false, false) => Self::Dir,
            (false, false, true, false, false, false, false) => Self::Symlink,
            (false, false, false, true, false, false, false) => Self::Socket,
            (false, false, false, false, true, false, false) => Self::Fifo,
            (false, false, false, false, false, true, false) => Self::CharDevice,
            (false, false, false, false, false, false, true) => Self::BlockDevice,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {}
