use cosmic::widget::image::Handle;

use crate::app::widgets::markdown;

pub async fn load_image(url: markdown::Url) -> Result<Handle, String> {
    use url::Url;

    let parsed = Url::parse(url.as_ref()).map_err(|e| e.to_string())?;

    if parsed.scheme() == "file" {
        let path = parsed.to_file_path().map_err(|_| "Invalid file path")?;
        Ok(Handle::from_path(path))
    } else if parsed.scheme() == "http" || parsed.scheme() == "https" {
        let bytes = reqwest::get(url.clone())
            .await
            .map_err(|e| e.to_string())?
            .bytes()
            .await
            .map_err(|e| e.to_string())?;

        let img = image::load_from_memory(&bytes)
            .map_err(|e| e.to_string())?
            .to_rgba8();

        Ok(Handle::from_rgba(img.width(), img.height(), img.into_raw()))
    } else {
        Err(format!("Unsupported URL scheme: {}", parsed.scheme()))
    }
}
