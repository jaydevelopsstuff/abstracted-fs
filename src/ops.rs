#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::future::Future;

use crate::{
    data::FileType,
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

            if let Err(error) = backend.create_dir(&to_dir_path).await {
                if !error.is_already_exists_error() {
                    return Err(error);
                }
            }

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

            if let Err(error) = backend.create_dir(&to_dir_path).await {
                if !error.is_already_exists_error() {
                    return Err(error);
                }
            }

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

#[derive(Debug, Clone, Default)]
pub struct TransitProgress {
    pub processed_bytes: u64,
    pub total_bytes: u64,
    pub processed_files: u64,
    pub total_files: u64,
    pub state: TransitState,
}

#[derive(Debug, Clone, Default)]
pub enum TransitState {
    #[default]
    Normal,
    Exists(TransferConflict),
    Other(Error),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TransferConflict {
    pub file_type: FileType,
    pub origin: String,
    pub destination: String,
}

// TODO: Retry
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum TransitProgressResponse {
    ContinueOrAbort,
    Skip,
    Overwrite,
    Abort,
}

async fn update_and_notify_progress_handler<Fut: Future<Output = TransitProgressResponse>>(
    progress: &mut TransitProgress,
    progress_handler: impl Fn(TransitProgress) -> Fut,
    error: Option<Error>,
    current_file_type: FileType,
    current_file_path: String,
    current_file_dest: String,
    current_file_size: Option<u64>,
) -> TransitProgressResponse {
    progress.state = if let Some(error) = error {
        if error.is_already_exists_error() {
            TransitState::Exists(TransferConflict {
                file_type: current_file_type,
                origin: current_file_path,
                destination: current_file_dest,
            })
        } else {
            TransitState::Other(error)
        }
    } else {
        TransitState::Normal
    };
    progress.processed_bytes += current_file_size.unwrap_or(0);

    progress_handler(progress.clone()).await
}

macro_rules! resolve_progress_handler_response {
    ($response:expr, $overwrite_op:expr, $progress:expr) => {
        match $response {
            TransitProgressResponse::ContinueOrAbort => match $progress.state {
                TransitState::Exists(conflict) => {
                    return Err(Error::FileAlreadyExists(conflict.destination))
                }
                TransitState::Other(error) => return Err(error),
                _ => (),
            },
            TransitProgressResponse::Skip => (),
            TransitProgressResponse::Overwrite => $overwrite_op().await?,
            TransitProgressResponse::Abort => return Ok(()),
        }
    };
}

pub async fn move_files_with_progress<
    S: AsRef<str>,
    Fut: Future<Output = TransitProgressResponse>,
>(
    backend: &dyn FSBackend,
    from: &[S],
    to: S,
    progress_handler: impl Fn(TransitProgress) -> Fut,
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
                    file.metadata.r#type,
                    file.path.clone(),
                    file_dest.clone(),
                    file.metadata.size,
                )
                .await;

                resolve_progress_handler_response!(
                    response,
                    || backend.move_file(&file.path, &file_dest, true),
                    progress
                );
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

            if let Err(error) = backend.create_dir(&to_dir_path).await {
                if !error.is_already_exists_error() {
                    return Err(error);
                }
            }

            for file in backend.read_dir(&dir.0).await? {
                match file.metadata.r#type {
                    FileType::File | FileType::Symlink => {
                        let file_dest = format!("{to_dir_path}/{}", file.name);
                        let result = backend.move_file(&file.path, &file_dest, false).await;

                        let response = update_and_notify_progress_handler(
                            &mut progress,
                            &progress_handler,
                            result.err(),
                            file.metadata.r#type,
                            file.path.clone(),
                            file_dest.clone(),
                            file.metadata.size,
                        )
                        .await;

                        resolve_progress_handler_response!(
                            response,
                            || backend.move_file(&file.path, &file_dest, true),
                            progress
                        );
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
    Fut: Future<Output = TransitProgressResponse>,
>(
    backend: &dyn FSBackend,
    from: &[S],
    to: S,
    progress_handler: impl Fn(TransitProgress) -> Fut,
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
            FileType::File => {
                let file_dest = format!("{to}/{}", extract_lowest_path_item(&file.path));
                let result = backend.copy_file(&file.path, &file_dest, false).await;

                let response = update_and_notify_progress_handler(
                    &mut progress,
                    &progress_handler,
                    result.err(),
                    file.metadata.r#type,
                    file.path.clone(),
                    file_dest.clone(),
                    file.metadata.size,
                )
                .await;

                resolve_progress_handler_response!(
                    response,
                    || backend.copy_file(&file.path, &file_dest, true),
                    progress
                );
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

            if let Err(error) = backend.create_dir(&to_dir_path).await {
                if !error.is_already_exists_error() {
                    return Err(error);
                }
            }

            for file in backend.read_dir(&dir.0).await? {
                match file.metadata.r#type {
                    FileType::File => {
                        let file_dest = format!("{to_dir_path}/{}", file.name);
                        let result = backend.copy_file(&file.path, &file_dest, false).await;

                        let response = update_and_notify_progress_handler(
                            &mut progress,
                            &progress_handler,
                            result.err(),
                            file.metadata.r#type,
                            file.path.clone(),
                            file_dest.clone(),
                            file.metadata.size,
                        )
                        .await;

                        resolve_progress_handler_response!(
                            response,
                            || backend.move_file(&file.path, &file_dest, true),
                            progress
                        );
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

pub async fn move_files_between<S: AsRef<str>>(
    from_backend: &dyn FSBackend,
    to_backend: &dyn FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    let to = to.as_ref();

    if !to_backend.exists(to).await? {
        return Err(Error::FileNonexistent(to.to_string()));
    }

    // The first string in the tuple represents the origin path, the second
    // represents the directories that encapsulate it relative to the lowest
    // directory in the from path.
    let mut dirs_to_duplicate: Vec<(String, String)> = vec![];

    for path in from {
        let path = path.as_ref();

        match from_backend.get_file_type(path).await? {
            FileType::File => {
                move_file_between(
                    from_backend,
                    to_backend,
                    path,
                    &format!("{to}/{}", extract_lowest_path_item(path)),
                    false,
                )
                .await?;
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

            if let Err(error) = to_backend.create_dir(&to_dir_path).await {
                if !error.is_already_exists_error() {
                    return Err(error);
                }
            }

            for file in from_backend.read_dir(&dir.0).await? {
                match file.metadata.r#type {
                    FileType::File => {
                        move_file_between(
                            from_backend,
                            to_backend,
                            &file.path,
                            &format!("{to_dir_path}/{}", file.name),
                            false,
                        )
                        .await?;
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
        from_backend.remove_dir(&dir).await?;
    }

    Ok(())
}

pub async fn copy_files_between<S: AsRef<str>>(
    from_backend: &dyn FSBackend,
    to_backend: &dyn FSBackend,
    from: &[S],
    to: S,
) -> Result<()> {
    let to = to.as_ref();

    if !to_backend.exists(to).await? {
        return Err(Error::FileNonexistent(to.to_string()));
    }

    // The first string in the tuple represents the origin path, the second
    // represents the directories that encapsulate it relative to the lowest
    // directory in the from path.
    let mut dirs_to_copy: Vec<(String, String)> = vec![];

    for path in from {
        let path = path.as_ref();

        match from_backend.get_file_type(path).await? {
            FileType::File | FileType::Symlink => {
                copy_file_between(
                    from_backend,
                    to_backend,
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

            if let Err(error) = to_backend.create_dir(&to_dir_path).await {
                if !error.is_already_exists_error() {
                    return Err(error);
                }
            }

            for file in from_backend.read_dir(&dir.0).await? {
                match file.metadata.r#type {
                    FileType::File | FileType::Symlink => {
                        copy_file_between(
                            from_backend,
                            to_backend,
                            &file.path,
                            &format!("{to_dir_path}/{}", file.name),
                            false,
                        )
                        .await?;
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

pub async fn move_files_between_with_progress<
    S: AsRef<str>,
    Fut: Future<Output = TransitProgressResponse>,
>(
    from_backend: &dyn FSBackend,
    to_backend: &dyn FSBackend,
    from: &[S],
    to: S,
    progress_handler: impl Fn(TransitProgress) -> Fut,
) -> Result<()> {
    let to = to.as_ref();

    if !to_backend.exists(to).await? {
        return Err(Error::FileNonexistent(to.to_string()));
    }

    let mut progress = TransitProgress {
        total_bytes: calculate_total_size(from_backend, from).await?,
        ..Default::default()
    };

    // The first string in the tuple represents the origin path, the second
    // represents the directories that encapsulate it relative to the lowest
    // directory in the from path.
    let mut dirs_to_duplicate: Vec<(String, String)> = vec![];

    for file in from_backend
        .retrieve_files(
            from.into_iter()
                .map(|path| path.as_ref().to_string())
                .collect(),
        )
        .await?
    {
        match file.metadata.r#type {
            FileType::File => {
                let file_dest = format!("{to}/{}", extract_lowest_path_item(&file.path));
                let result =
                    move_file_between(from_backend, to_backend, &file.path, &file_dest, false)
                        .await;

                let response = update_and_notify_progress_handler(
                    &mut progress,
                    &progress_handler,
                    result.err(),
                    file.metadata.r#type,
                    file.path.clone(),
                    file_dest.clone(),
                    file.metadata.size,
                )
                .await;

                resolve_progress_handler_response!(
                    response,
                    || move_file_between(from_backend, to_backend, &file.path, &file_dest, true),
                    progress
                );
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

            if let Err(error) = to_backend.create_dir(&to_dir_path).await {
                if !error.is_already_exists_error() {
                    return Err(error);
                }
            }

            for file in from_backend.read_dir(&dir.0).await? {
                match file.metadata.r#type {
                    FileType::File | FileType::Symlink => {
                        let file_dest = format!("{to_dir_path}/{}", file.name);
                        let result = move_file_between(
                            from_backend,
                            to_backend,
                            &file.path,
                            &file_dest,
                            false,
                        )
                        .await;

                        let response = update_and_notify_progress_handler(
                            &mut progress,
                            &progress_handler,
                            result.err(),
                            file.metadata.r#type,
                            file.path.clone(),
                            file_dest.clone(),
                            file.metadata.size,
                        )
                        .await;

                        resolve_progress_handler_response!(
                            response,
                            || move_file_between(
                                from_backend,
                                to_backend,
                                &file.path,
                                &file_dest,
                                true
                            ),
                            progress
                        );
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
        from_backend.remove_dir(&dir).await?;
    }

    Ok(())
}

pub async fn copy_files_between_with_progress<
    S: AsRef<str>,
    Fut: Future<Output = TransitProgressResponse>,
>(
    from_backend: &dyn FSBackend,
    to_backend: &dyn FSBackend,
    from: &[S],
    to: S,
    progress_handler: impl Fn(TransitProgress) -> Fut,
) -> Result<()> {
    let to = to.as_ref();

    if !to_backend.exists(to).await? {
        return Err(Error::FileNonexistent(to.to_string()));
    }

    let mut progress = TransitProgress {
        total_bytes: calculate_total_size(from_backend, from).await?,
        ..Default::default()
    };

    // The first string in the tuple represents the origin path, the second
    // represents the directories that encapsulate it relative to the lowest
    // directory in the from path.
    let mut dirs_to_copy: Vec<(String, String)> = vec![];

    for file in from_backend
        .retrieve_files(
            from.into_iter()
                .map(|path| path.as_ref().to_string())
                .collect(),
        )
        .await?
    {
        match file.metadata.r#type {
            FileType::File => {
                let file_dest = format!("{to}/{}", extract_lowest_path_item(&file.path));
                let result =
                    copy_file_between(from_backend, to_backend, &file.path, &file_dest, false)
                        .await;

                let response = update_and_notify_progress_handler(
                    &mut progress,
                    &progress_handler,
                    result.err(),
                    file.metadata.r#type,
                    file.path.clone(),
                    file_dest.clone(),
                    file.metadata.size,
                )
                .await;

                resolve_progress_handler_response!(
                    response,
                    || copy_file_between(from_backend, to_backend, &file.path, &file_dest, true),
                    progress
                );
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

            if let Err(error) = to_backend.create_dir(&to_dir_path).await {
                if !error.is_already_exists_error() {
                    return Err(error);
                }
            }

            for file in from_backend.read_dir(&dir.0).await? {
                match file.metadata.r#type {
                    FileType::File => {
                        let file_dest = format!("{to_dir_path}/{}", file.name);
                        let result = from_backend.copy_file(&file.path, &file_dest, false).await;

                        let response = update_and_notify_progress_handler(
                            &mut progress,
                            &progress_handler,
                            result.err(),
                            file.metadata.r#type,
                            file.path.clone(),
                            file_dest.clone(),
                            file.metadata.size,
                        )
                        .await;

                        resolve_progress_handler_response!(
                            response,
                            || copy_file_between(
                                from_backend,
                                to_backend,
                                &file.path,
                                &file_dest,
                                true
                            ),
                            progress
                        );
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
