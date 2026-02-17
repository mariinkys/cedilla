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
    dbg!(&path);
    tokio::fs::write(&path, content)
        .await
        .map_err(|e| anywho!("{}", e))?;

    Ok(path)
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
