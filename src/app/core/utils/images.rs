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
