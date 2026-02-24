use cosmic::widget::{image::Handle, markdown};

pub async fn load_image(url: markdown::Uri) -> Result<Handle, anywho::Error> {
    use url::Url;

    let parsed = Url::parse(url.as_ref()).map_err(|e| anywho::anywho!("{e}"))?;

    if parsed.scheme() == "file" {
        let path = parsed
            .to_file_path()
            .map_err(|_| anywho::anywho!("Invalid file path"))?;
        Ok(Handle::from_path(path))
    } else if parsed.scheme() == "http" || parsed.scheme() == "https" {
        let bytes = reqwest::get(url.clone())
            .await
            .map_err(|e| anywho::anywho!("{e}"))?
            .bytes()
            .await
            .map_err(|e| anywho::anywho!("{e}"))?;

        let img = image::load_from_memory(&bytes)
            .map_err(|e| anywho::anywho!("{e}"))?
            .to_rgba8();

        Ok(Handle::from_rgba(img.width(), img.height(), img.into_raw()))
    } else {
        Err(anywho::anywho!(
            "Unsupported URL scheme: {}",
            parsed.scheme()
        ))
    }
}
