// SPDX-License-Identifier: GPL-3.0

use std::{collections::HashSet, path::PathBuf};

use cosmic::{Action, Task};
use frostmark::MarkState;

use crate::app::Message;

#[derive(Debug, Clone)]
pub struct Image {
    pub bytes: Vec<u8>,
    pub url: String,
    #[allow(unused)]
    pub is_svg: bool,
}

async fn load_image(url: String, base_path: Option<PathBuf>) -> Result<Image, anywho::Error> {
    use url::Url;

    let resolved_url = if url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("file://")
    {
        url.clone()
    } else {
        // Relative path — resolve against base_path
        let base = base_path.ok_or_else(|| anywho::anywho!("No base path for relative URL"))?;
        let base_dir = if base.is_dir() {
            base
        } else {
            base.parent()
                .map(PathBuf::from)
                .ok_or_else(|| anywho::anywho!("No parent directory"))?
        };
        let resolved = base_dir.join(&url);
        format!("file://{}", resolved.display())
    };

    let parsed = Url::parse(&resolved_url).map_err(|e| anywho::anywho!("{e}"))?;

    if parsed.scheme() == "file" {
        let path = parsed
            .to_file_path()
            .map_err(|_| anywho::anywho!("Invalid file path"))?;

        let bytes = std::fs::read(&path)?;

        let is_svg = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("svg"))
            .unwrap_or(false);

        Ok(Image { bytes, url, is_svg })
    } else if parsed.scheme() == "http" || parsed.scheme() == "https" {
        let response = reqwest::get(url.clone())
            .await
            .map_err(|e| anywho::anywho!("{e}"))?;

        let is_svg = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|ct| ct.contains("svg"))
            .unwrap_or(false)
            || url.trim_end().to_lowercase().ends_with(".svg");

        let bytes = response
            .bytes()
            .await
            .map_err(|e| anywho::anywho!("{e}"))?
            .to_vec();

        Ok(Image { bytes, url, is_svg })
    } else {
        Err(anywho::anywho!(
            "Unsupported URL scheme: {}",
            parsed.scheme()
        ))
    }
}

pub fn download_images(
    markstate: &mut MarkState,
    images_in_progress: &mut HashSet<String>,
    base_path: &Option<PathBuf>,
) -> Task<Action<Message>> {
    Task::batch(markstate.find_image_links().into_iter().map(|url| {
        if images_in_progress.insert(url.clone()) {
            Task::perform(load_image(url, base_path.clone()), Message::ImageDownloaded)
                .map(cosmic::action::app)
        } else {
            Task::none()
        }
    }))
}

/// Saves already-encoded image bytes (as read from the clipboard) to `target_dir` with a
/// unique timestamped file name and returns the file name.
pub fn save_image_bytes(
    target_dir: &std::path::Path,
    bytes: &[u8],
    extension: &str,
) -> Result<String, anywho::Error> {
    std::fs::create_dir_all(target_dir)?;

    let timestamp = jiff::Zoned::now().strftime("%Y%m%d_%H%M%S").to_string();
    let mut file_name = format!("image_{timestamp}.{extension}");
    let mut file_path = target_dir.join(&file_name);
    let mut counter = 1;
    while file_path.exists() {
        file_name = format!("image_{timestamp}_{counter}.{extension}");
        file_path = target_dir.join(&file_name);
        counter += 1;
    }

    std::fs::write(&file_path, bytes)
        .map_err(|e| anywho::anywho!("Failed to save image file: {e}"))?;

    Ok(file_name)
}

/// Checks if the given text contains a local image file path, file:// URI, or web image URL,
/// copies local files to target_dir, and returns the image reference.
pub fn copy_image_from_text_path(
    target_dir: &std::path::Path,
    text: &str,
) -> Result<Option<String>, anywho::Error> {
    for raw_line in text.lines() {
        let line = raw_line.trim().trim_matches('"').trim_matches('\'');
        if line.is_empty() || line == "copy" || line == "cut" {
            continue;
        }

        // Check if it's a web URL for an image
        #[allow(clippy::collapsible_if)]
        if line.starts_with("http://") || line.starts_with("https://") {
            if let Ok(parsed_url) = url::Url::parse(line) {
                let path = parsed_url.path();
                if let Some(ext) = std::path::Path::new(path)
                    .extension()
                    .and_then(|e| e.to_str())
                {
                    let ext_lower = ext.to_lowercase();
                    if is_image_extension(&ext_lower) {
                        return Ok(Some(line.to_string()));
                    }
                }
            }
        }

        let path_str = if let Some(stripped) = line.strip_prefix("file://") {
            percent_encoding::percent_decode_str(stripped)
                .decode_utf8()
                .map(|c| c.to_string())
                .unwrap_or_else(|_| stripped.to_string())
        } else {
            line.to_string()
        };

        let src_path = std::path::Path::new(&path_str);
        #[allow(clippy::collapsible_if)]
        if src_path.exists() && src_path.is_file() {
            if let Some(ext) = src_path.extension().and_then(|e| e.to_str()) {
                let ext_lower = ext.to_lowercase();
                if is_image_extension(&ext_lower) {
                    std::fs::create_dir_all(target_dir)?;
                    let file_stem = src_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("image");
                    let mut file_name = format!("{file_stem}.{ext_lower}");
                    let mut dest_path = target_dir.join(&file_name);
                    let mut counter = 1;
                    if dest_path == src_path {
                        return Ok(Some(file_name));
                    }
                    while dest_path.exists() {
                        file_name = format!("{file_stem}_{counter}.{ext_lower}");
                        dest_path = target_dir.join(&file_name);
                        counter += 1;
                    }
                    std::fs::copy(src_path, &dest_path)
                        .map_err(|e| anywho::anywho!("Failed to copy image file: {e}"))?;
                    return Ok(Some(file_name));
                }
            }
        }
    }
    Ok(None)
}

fn is_image_extension(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "tiff" | "avif"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_image_from_text_path() {
        let base_dir =
            std::env::temp_dir().join(format!("cedilla_copy_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base_dir);
        std::fs::create_dir_all(&base_dir).unwrap();

        let src_file = base_dir.join("sample.png");
        std::fs::write(&src_file, b"fake png data").unwrap();

        let target_dir = base_dir.join("target");

        // Test with file:// URI
        let uri = format!("file://{}", src_file.display());
        let res = copy_image_from_text_path(&target_dir, &uri).expect("copy failed");
        assert_eq!(res, Some("sample.png".to_string()));
        assert!(target_dir.join("sample.png").exists());

        // Test with normal path string
        let path_str = src_file.to_str().unwrap();
        let res2 = copy_image_from_text_path(&target_dir, path_str).expect("copy failed");
        assert_eq!(res2, Some("sample_1.png".to_string()));
        assert!(target_dir.join("sample_1.png").exists());

        // Test with GNOME/Cosmic copied files format (copy\nfile://...)
        let gnome_clipboard = format!("copy\nfile://{}", src_file.display());
        let res3 = copy_image_from_text_path(&target_dir, &gnome_clipboard).expect("copy failed");
        assert_eq!(res3, Some("sample_2.png".to_string()));
        assert!(target_dir.join("sample_2.png").exists());

        // Test with quoted path
        let quoted = format!("\"{}\"", src_file.display());
        let res4 = copy_image_from_text_path(&target_dir, &quoted).expect("copy failed");
        assert_eq!(res4, Some("sample_3.png".to_string()));
        assert!(target_dir.join("sample_3.png").exists());

        // Test with web URL
        let web_url = "https://example.com/assets/banner.png";
        let res5 = copy_image_from_text_path(&target_dir, web_url).expect("url check failed");
        assert_eq!(res5, Some(web_url.to_string()));

        // Test with plain non-image text (should return None)
        let plain_text = "just some random text without images";
        let res6 = copy_image_from_text_path(&target_dir, plain_text).expect("check failed");
        assert_eq!(res6, None);

        let _ = std::fs::remove_dir_all(&base_dir);
    }
}
