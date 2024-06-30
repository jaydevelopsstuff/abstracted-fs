use crate::{
    data::{File, FileType},
    error::{Error, Result},
    util::extract_lowest_path_item,
    FSBackend,
};

pub async fn calculate_total_size<S: AsRef<str>>(
    backend: &dyn FSBackend,
    paths: &[S],
) -> Result<u64> {
    let mut total_size = 0;

    let mut dirs_to_process = vec![];

    for file in backend
        .retrieve_files(
            paths
                .into_iter()
                .map(|path| path.as_ref().to_string())
                .collect(),
        )
        .await?
    {
        if file.metadata.r#type == FileType::Dir {
            dirs_to_process.push(file.path);
        } else {
            total_size += file.metadata.size.unwrap_or(0);
        }
    }

    while !dirs_to_process.is_empty() {
        let mut new_dirs_to_process = vec![];

        for dir in dirs_to_process {
            for file in backend.read_dir(&dir).await? {
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

pub async fn remove_all<S: AsRef<str>>(backend: &dyn FSBackend, paths: &[S]) -> Result<()> {
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

pub async fn move_files<S: AsRef<str>>(backend: &dyn FSBackend, from: &[S], to: S) -> Result<()> {
    let to = to.as_ref();

    if !backend.exists(to).await? {
        return Err(Error::FileNonexistent(to.to_string()));
    }

    // The first string in the tuple represents the origin path, the second
    // represents the directories that encapsulate it relative to the lowest
    // directory in the from path.
    let mut dirs_to_duplicate: Vec<(String, String)> = vec![];

    for path in from {
        let path = path.as_ref();

        match backend.get_file_type(path).await? {
            FileType::File | FileType::Symlink => {
                backend
                    .move_file(
                        path,
                        &format!("{to}/{}", extract_lowest_path_item(path)),
                        false,
                    )
                    .await?
            }
            FileType::Dir => dirs_to_duplicate.push((path.into(), "/".into())),
            t => return Err(Error::CannotCopyOrMoveFileType(t)),
        }
    }

    let mut dirs_to_remove = vec![];
    while !dirs_to_duplicate.is_empty() {
        let mut new_dirs_to_copy = vec![];

        for dir in dirs_to_duplicate {
            let parent_dirs = dir.1;
            let dir_name = extract_lowest_path_item(&dir.0);
            let to_dir_path = format!("{to}{parent_dirs}{dir_name}");

            backend.create_dir(&to_dir_path).await?;

            for file in backend.read_dir(&dir.0).await? {
                match file.metadata.r#type {
                    FileType::File | FileType::Symlink => {
                        backend
                            .move_file(&file.path, &format!("{to_dir_path}/{}", file.name), false)
                            .await?
                    }
                    FileType::Dir => {
                        new_dirs_to_copy.push((file.path, format!("{parent_dirs}{dir_name}/")));
                    }
                    t => return Err(Error::CannotCopyOrMoveFileType(t)),
                }
            }

            dirs_to_remove.push(dir.0);
        }

        dirs_to_duplicate = new_dirs_to_copy
    }

    while let Some(dir) = dirs_to_remove.pop() {
        backend.remove_dir(&dir).await?;
    }

    Ok(())
}

pub async fn copy_files<S: AsRef<str>>(backend: &dyn FSBackend, from: &[S], to: S) -> Result<()> {
    let to = to.as_ref();

    if !backend.exists(to).await? {
        return Err(Error::FileNonexistent(to.to_string()));
    }

    // The first string in the tuple represents the origin path, the second
    // represents the directories that encapsulate it relative to the lowest
    // directory in the from path.
    let mut dirs_to_copy: Vec<(String, String)> = vec![];

    for path in from {
        let path = path.as_ref();

        match backend.get_file_type(path).await? {
            FileType::File | FileType::Symlink => {
                backend
                    .copy_file(
                        path,
                        &format!("{to}/{}", extract_lowest_path_item(path)),
                        false,
                    )
                    .await?
            }
            FileType::Dir => dirs_to_copy.push((path.into(), "/".into())),
            t => return Err(Error::CannotCopyOrMoveFileType(t)),
        }
    }

    while !dirs_to_copy.is_empty() {
        let mut new_dirs_to_copy = vec![];

        for dir in dirs_to_copy {
            let parent_dirs = dir.1;
            let dir_name = extract_lowest_path_item(&dir.0);
            let to_dir_path = format!("{to}{parent_dirs}{dir_name}");

            backend.create_dir(&to_dir_path).await?;

            for file in backend.read_dir(&dir.0).await? {
                match file.metadata.r#type {
                    FileType::File | FileType::Symlink => {
                        backend
                            .copy_file(&file.path, &format!("{to_dir_path}/{}", file.name), false)
                            .await?
                    }
                    FileType::Dir => {
                        new_dirs_to_copy.push((file.path, format!("{parent_dirs}{dir_name}/")));
                    }
                    t => return Err(Error::CannotCopyOrMoveFileType(t)),
                }
            }
        }

        dirs_to_copy = new_dirs_to_copy
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct TransitProgress {
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub processed_files: u64,
    pub total_files: u64,
    pub state: TransitState,
}

#[derive(Debug, Default)]
pub enum TransitState {
    #[default]
    Normal,
    Exists(File, String),
    Other(Error),
}

#[derive(Debug, Clone)]
pub enum TransitProgressResponse {
    ContinueOrAbort,
    Skip,
    Overwrite,
    Abort,
}

fn update_and_notify_progress_handler<F: Fn(&TransitProgress) -> TransitProgressResponse>(
    progress: &mut TransitProgress,
    progress_handler: &F,
    error: Option<Error>,
    current_file: File,
    current_file_dest: String,
) -> TransitProgressResponse {
    progress.state = if let Some(error) = error {
        if error.is_already_exists_error() {
            TransitState::Exists(current_file.clone(), current_file_dest)
        } else {
            TransitState::Other(error)
        }
    } else {
        TransitState::Normal
    };
    progress.processed_bytes += current_file.metadata.size.unwrap_or(0);

    progress_handler(progress)
}

macro_rules! resolve_progress_handler_response {
    ($response:expr, $backend:expr, $progress:expr) => {
        match $response {
            TransitProgressResponse::ContinueOrAbort => {
                if let TransitState::Other(error) = $progress.state {
                    return Err(error);
                }
            }
            TransitProgressResponse::Skip => (),
            TransitProgressResponse::Overwrite => (),
            TransitProgressResponse::Abort => return Ok(()),
        }
    };
}

pub async fn move_files_with_progress<
    S: AsRef<str>,
    F: Fn(&TransitProgress) -> TransitProgressResponse,
>(
    backend: &dyn FSBackend,
    from: &[S],
    to: S,
    progress_handler: F,
) -> Result<()> {
    let to = to.as_ref();

    if !backend.exists(to).await? {
        return Err(Error::FileNonexistent(to.to_string()));
    }

    let mut progress = TransitProgress {
        total_bytes: calculate_total_size(backend, from).await?,
        ..Default::default()
    };

    // The first string in the tuple represents the origin path, the second
    // represents the directories that encapsulate it relative to the lowest
    // directory in the from path.
    let mut dirs_to_duplicate: Vec<(String, String)> = vec![];

    for file in backend
        .retrieve_files(
            from.into_iter()
                .map(|path| path.as_ref().to_string())
                .collect(),
        )
        .await?
    {
        match file.metadata.r#type {
            FileType::File | FileType::Symlink => {
                let file_dest = format!("{to}/{}", extract_lowest_path_item(&file.path));
                let result = backend.move_file(&file.path, &file_dest, false).await;

                let response = update_and_notify_progress_handler(
                    &mut progress,
                    &progress_handler,
                    result.err(),
                    file,
                    file_dest,
                );

                resolve_progress_handler_response!(response, backend, progress);
            }
            FileType::Dir => dirs_to_duplicate.push((file.path, "/".into())),
            t => return Err(Error::CannotCopyOrMoveFileType(t)),
        }
    }

    let mut dirs_to_remove = vec![];
    while !dirs_to_duplicate.is_empty() {
        let mut new_dirs_to_copy = vec![];

        for dir in dirs_to_duplicate {
            let parent_dirs = dir.1;
            let dir_name = extract_lowest_path_item(&dir.0);
            let to_dir_path = format!("{to}{parent_dirs}{dir_name}");

            backend.create_dir(&to_dir_path).await?;

            for file in backend.read_dir(&dir.0).await? {
                match file.metadata.r#type {
                    FileType::File | FileType::Symlink => {
                        let file_dest = format!("{to_dir_path}/{}", file.name);
                        let result = backend.move_file(&file.path, &file_dest, false).await;

                        let response = update_and_notify_progress_handler(
                            &mut progress,
                            &progress_handler,
                            result.err(),
                            file,
                            file_dest,
                        );

                        resolve_progress_handler_response!(response, backend, progress);
                    }
                    FileType::Dir => {
                        new_dirs_to_copy.push((file.path, format!("{parent_dirs}{dir_name}/")));
                    }
                    t => return Err(Error::CannotCopyOrMoveFileType(t)),
                }
            }

            dirs_to_remove.push(dir.0);
        }

        dirs_to_duplicate = new_dirs_to_copy
    }

    while let Some(dir) = dirs_to_remove.pop() {
        backend.remove_dir(&dir).await?;
    }

    Ok(())
}

pub async fn copy_files_with_progress<
    S: AsRef<str>,
    F: Fn(&TransitProgress) -> TransitProgressResponse,
>(
    backend: &dyn FSBackend,
    from: &[S],
    to: S,
    progress_handler: F,
) -> Result<()> {
    let to = to.as_ref();

    if !backend.exists(to).await? {
        return Err(Error::FileNonexistent(to.to_string()));
    }

    let mut progress = TransitProgress {
        total_bytes: calculate_total_size(backend, from).await?,
        ..Default::default()
    };

    // The first string in the tuple represents the origin path, the second
    // represents the directories that encapsulate it relative to the lowest
    // directory in the from path.
    let mut dirs_to_copy: Vec<(String, String)> = vec![];

    for file in backend
        .retrieve_files(
            from.into_iter()
                .map(|path| path.as_ref().to_string())
                .collect(),
        )
        .await?
    {
        match file.metadata.r#type {
            FileType::File | FileType::Symlink => {
                let file_dest = format!("{to}/{}", extract_lowest_path_item(&file.path));
                let result = backend.copy_file(&file.path, &file_dest, false).await;

                let response = update_and_notify_progress_handler(
                    &mut progress,
                    &progress_handler,
                    result.err(),
                    file,
                    file_dest,
                );

                resolve_progress_handler_response!(response, backend, progress);
            }
            FileType::Dir => dirs_to_copy.push((file.path, "/".into())),
            t => return Err(Error::CannotCopyOrMoveFileType(t)),
        }
    }

    while !dirs_to_copy.is_empty() {
        let mut new_dirs_to_copy = vec![];

        for dir in dirs_to_copy {
            let parent_dirs = dir.1;
            let dir_name = extract_lowest_path_item(&dir.0);
            let to_dir_path = format!("{to}{parent_dirs}{dir_name}");

            backend.create_dir(&to_dir_path).await?;

            for file in backend.read_dir(&dir.0).await? {
                match file.metadata.r#type {
                    FileType::File | FileType::Symlink => {
                        let file_dest = format!("{to_dir_path}/{}", file.name);
                        let result = backend.copy_file(&file.path, &file_dest, false).await;

                        let response = update_and_notify_progress_handler(
                            &mut progress,
                            &progress_handler,
                            result.err(),
                            file,
                            file_dest,
                        );

                        resolve_progress_handler_response!(response, backend, progress);
                    }
                    FileType::Dir => {
                        new_dirs_to_copy.push((file.path, format!("{parent_dirs}{dir_name}/")));
                    }
                    t => return Err(Error::CannotCopyOrMoveFileType(t)),
                }
            }
        }

        dirs_to_copy = new_dirs_to_copy
    }

    Ok(())
}

async fn move_file_between(
    from_backend: &dyn FSBackend,
    to_backend: &dyn FSBackend,
    from: &str,
    to: &str,
    overwrite: bool,
) -> Result<()> {
    copy_file_between(from_backend, to_backend, from, to, overwrite).await?;
    from_backend.remove_file(from).await?;
    Ok(())
}

async fn copy_file_between(
    from_backend: &dyn FSBackend,
    to_backend: &dyn FSBackend,
    from: &str,
    to: &str,
    overwrite: bool,
) -> Result<()> {
    let contents = from_backend.retrieve_file_content(from).await?;
    to_backend
        .create_file(to, overwrite, Some(&contents))
        .await?;
    Ok(())
}

pub async fn move_files_between<S: AsRef<str>>(
    from_backend: &dyn FSBackend,
    to_backend: &dyn FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}

pub async fn copy_files_between<S: AsRef<str>>(
    from_backend: &dyn FSBackend,
    to_backend: &dyn FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}

pub async fn move_files_between_with_progress<
    S: AsRef<str>,
    F: FnMut(&TransitProgress) -> TransitProgressResponse,
>(
    from_backend: &dyn FSBackend,
    to_backend: &dyn FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}

pub async fn copy_files_between_with_progress<
    S: AsRef<str>,
    F: FnMut(&TransitProgress) -> TransitProgressResponse,
>(
    from_backend: &dyn FSBackend,
    to_backend: &dyn FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    todo!()
}
