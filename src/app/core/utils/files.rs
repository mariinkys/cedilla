// SPDX-License-Identifier: GPL-3.0

use anywho::anywho;
use cosmic::dialog::{ashpd::desktop::file_chooser::SelectedFiles, file_chooser::FileFilter};
use std::{path::PathBuf, sync::Arc};

pub async fn load_file(path: PathBuf) -> Result<(PathBuf, Arc<String>), anywho::Error> {
    let contents = tokio::fs::read_to_string(&path)
        .await
        .map(Arc::new)
        .map_err(|e| anywho!("{}", e))?;

    Ok((path, contents))
}

pub async fn save_file(path: PathBuf, content: String) -> Result<PathBuf, anywho::Error> {
    tokio::fs::write(&path, content)
        .await
        .map_err(|e| anywho!("{}", e))?;

    Ok(path)
}

pub async fn move_vault(new_path: PathBuf, old_path: PathBuf) -> Result<PathBuf, anywho::Error> {
    if !old_path.exists() {
        return Err(anywho!(
            "Source vault path does not exist: {}",
            old_path.display()
        ));
    }

    if !old_path.is_dir() {
        return Err(anywho!(
            "Source vault path is not a directory: {}",
            old_path.display()
        ));
    }

    // prevent moving the vault into one of its own subdirectories
    if new_path.starts_with(&old_path) {
        return Err(anywho!(
            "Destination '{}' is inside the current vault '{}'. Cannot move a vault into itself.",
            new_path.display(),
            old_path.display()
        ));
    }

    let destination = new_path.join("vault");

    // if destination.exists() {
    //     return Err(anywho!(
    //         "A folder named '{}' already exists at the destination: {}",
    //         vault_name.to_string_lossy(),
    //         new_path.display()
    //     ));
    // }

    tokio::fs::create_dir_all(&new_path)
        .await
        .map_err(|e| anywho!("Failed to create destination directory: {}", e))?;

    if tokio::fs::rename(&old_path, &destination).await.is_ok() {
        return Ok(destination);
    }

    // fall back to recursive copy + delete when different filesystems
    copy_dir_recursive(&old_path, &destination).await?;

    tokio::fs::remove_dir_all(&old_path)
        .await
        .map_err(|e| anywho!("Vault copied but failed to remove old location: {}", e))?;

    Ok(destination)
}

// Recursively copies a directory tree from `src` to `dst`.
async fn copy_dir_recursive(src: &PathBuf, dst: &PathBuf) -> Result<(), anywho::Error> {
    tokio::fs::create_dir_all(dst)
        .await
        .map_err(|e| anywho!("Failed to create directory {}: {}", dst.display(), e))?;

    let mut entries = tokio::fs::read_dir(src)
        .await
        .map_err(|e| anywho!("Failed to read directory {}: {}", src.display(), e))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| anywho!("Failed to read directory entry: {}", e))?
    {
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        let file_type = entry
            .file_type()
            .await
            .map_err(|e| anywho!("Failed to get file type for {}: {}", src_path.display(), e))?;

        if file_type.is_dir() {
            Box::pin(copy_dir_recursive(&src_path, &dst_path)).await?;
        } else if file_type.is_symlink() {
            let link_target = tokio::fs::read_link(&src_path)
                .await
                .map_err(|e| anywho!("Failed to read symlink {}: {}", src_path.display(), e))?;
            tokio::fs::symlink(link_target, &dst_path)
                .await
                .map_err(|e| anywho!("Failed to create symlink {}: {}", dst_path.display(), e))?;
        } else {
            tokio::fs::copy(&src_path, &dst_path).await.map_err(|e| {
                anywho!(
                    "Failed to copy {} to {}: {}",
                    src_path.display(),
                    dst_path.display(),
                    e
                )
            })?;
        }
    }

    Ok(())
}

/// Open a system dialog to select a markdown file, returns the selected file (if any)
pub async fn open_markdown_file_picker() -> Option<String> {
    let result = SelectedFiles::open_file()
        .title("Select Markdown File")
        .accept_label("Open")
        .modal(true)
        .multiple(false)
        .filter(
            FileFilter::new("Markdown Files")
                .glob("*.md")
                .glob("*.txt")
                .glob("*.MD"),
        )
        .send()
        .await
        .unwrap()
        .response();

    if let Ok(result) = result {
        result
            .uris()
            .iter()
            .map(|file| file.path().to_string())
            .collect::<Vec<String>>()
            .first()
            .cloned()
    } else {
        None
    }
}

/// Open a system dialog to select where to save a markdown file, returns the selected file (if any)
pub async fn open_markdown_file_saver(vault_path: String) -> Option<String> {
    let result = SelectedFiles::save_file()
        .title("Save File")
        .accept_label("Save")
        .modal(true)
        .current_folder(vault_path)
        .unwrap_or_default()
        .filter(
            FileFilter::new("Markdown Files")
                .glob("*.md")
                .glob("*.txt")
                .glob("*.MD"),
        )
        .send()
        .await
        .unwrap()
        .response();

    if let Ok(result) = result {
        result
            .uris()
            .iter()
            .map(|file| file.path().to_string())
            .collect::<Vec<String>>()
            .first()
            .cloned()
    } else {
        None
    }
}

/// Open a system dialog to select where to save a markdown file, returns the selected file (if any)
pub async fn open_pdf_file_saver() -> Option<String> {
    let result = SelectedFiles::save_file()
        .title("Save File")
        .accept_label("Save")
        .modal(true)
        .filter(FileFilter::new("PDF Files").glob("*.pdf"))
        .send()
        .await
        .unwrap()
        .response();

    if let Ok(result) = result {
        result
            .uris()
            .iter()
            .map(|file| file.path().to_string())
            .collect::<Vec<String>>()
            .first()
            .cloned()
    } else {
        None
    }
}

/// Open a system dialog to select a folder
pub async fn open_folder_picker(vault_path: String) -> Option<String> {
    let result = SelectedFiles::open_file()
        .title("Pick Folder")
        .accept_label("Pick")
        .modal(true)
        .directory(true)
        .current_folder(vault_path)
        .unwrap_or_default()
        .send()
        .await
        .unwrap()
        .response();

    if let Ok(result) = result {
        result
            .uris()
            .iter()
            .map(|file| {
                percent_encoding::percent_decode_str(file.path())
                    .decode_utf8_lossy()
                    .into_owned()
            })
            .collect::<Vec<String>>()
            .first()
            .cloned()
    } else {
        None
    }
}
