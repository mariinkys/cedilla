use std::collections::HashSet;

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

async fn load_image(url: String) -> Result<Image, anywho::Error> {
    use url::Url;
    let parsed = Url::parse(url.as_ref()).map_err(|e| anywho::anywho!("{e}"))?;

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
        let bytes = reqwest::get(url.clone())
            .await
            .map_err(|e| anywho::anywho!("{e}"))?
            .bytes()
            .await
            .map_err(|e| anywho::anywho!("{e}"))?
            .to_vec();

        let is_svg = url.trim_end().to_lowercase().ends_with(".svg");

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
) -> Task<Action<Message>> {
    Task::batch(markstate.find_image_links().into_iter().map(|url| {
        if images_in_progress.insert(url.clone()) {
            Task::perform(load_image(url), Message::ImageDownloaded).map(cosmic::action::app)
        } else {
            Task::none()
        }
    }))
}
