use std::{
    path::{Path, PathBuf},
    sync::OnceLock,
};

// We do this so that we don't have to recompile the regex every time
static MD_IMG_RE: OnceLock<regex::Regex> = OnceLock::new();
static HTML_IMG_RE: OnceLock<regex::Regex> = OnceLock::new();

fn md_img_re() -> &'static regex::Regex {
    MD_IMG_RE.get_or_init(|| regex::Regex::new(r"!\[([^\]]*)\]\((\.\/[^)]+)\)").unwrap())
}

fn html_img_re() -> &'static regex::Regex {
    HTML_IMG_RE
        .get_or_init(|| regex::Regex::new(r#"<img([^>]*?)src="(\./[^"]+)"([^>]*?)/?>"#).unwrap())
}

async fn embed_local_images(content: &str, base_dir: &Path) -> String {
    let md_re = md_img_re();
    let html_re = html_img_re();

    let mut cache: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();

    for cap in md_re.captures_iter(content) {
        cache.entry(cap[2].to_string()).or_insert(None);
    }
    for cap in html_re.captures_iter(content) {
        cache.entry(cap[2].to_string()).or_insert(None);
    }

    // read all images
    for (path, slot) in cache.iter_mut() {
        *slot = read_image_as_data_uri(base_dir, path).await;
    }

    // apply markdown replacements
    let result = md_re.replace_all(content, |cap: &regex::Captures| {
        let alt = &cap[1];
        let path = &cap[2];
        match cache.get(path).and_then(|v| v.as_ref()) {
            Some(data_uri) => format!("![{}]({})", alt, data_uri),
            None => cap[0].to_string(),
        }
    });

    // apply HTML replacements
    let result = html_re.replace_all(&result, |cap: &regex::Captures| {
        let before = &cap[1];
        let path = &cap[2];
        let after = &cap[3];
        match cache.get(path).and_then(|v| v.as_ref()) {
            Some(data_uri) => format!("<img{}src=\"{}\"{}/>", before, data_uri, after),
            None => cap[0].to_string(),
        }
    });

    result.into_owned()
}

async fn read_image_as_data_uri(base_dir: &Path, path: &str) -> Option<String> {
    use base64::{Engine, engine::general_purpose};

    let image_path = base_dir.join(path.trim_start_matches("./"));
    if !image_path.exists() {
        return None;
    }

    let mime = match image_path.extension().and_then(|e| e.to_str()) {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        _ => return None,
    };

    let bytes = tokio::fs::read(&image_path).await.ok()?;
    let b64 = general_purpose::STANDARD.encode(&bytes);
    Some(format!("data:{};base64,{}", mime, b64))
}

pub async fn export_pdf(
    client: gotenberg_pdf::Client,
    file_path: Option<PathBuf>,
    file_content: String,
    file_destination_path: String,
) -> Result<(), anywho::Error> {
    use pulldown_cmark::{Options, Parser, html};

    let (title, processed_content) = match &file_path {
        Some(path) => {
            let title = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Document")
                .to_string();
            let base_dir = path.parent().unwrap_or(Path::new("."));
            let processed = embed_local_images(&file_content, base_dir).await;
            (title, processed)
        }
        None => ("Document".to_string(), file_content.clone()),
    };

    // we convert the markdown to html ourselves
    let mut md_html = String::new();
    let parser = Parser::new_ext(&processed_content, Options::all());
    html::push_html(&mut md_html, parser);

    let full_html = format!(
        r#"<!doctype html>
            <html lang="en">
            <head>
                <meta charset="utf-8">
                <title>{title}</title>
                <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/styles/default.min.css">
                <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/highlight.min.js"></script>
                <script>hljs.highlightAll();</script>
                <style>
                    img {{
                    max-width: 100%;
                    height: auto;
                    }}
                </style>
            </head>
            <body>
                {md_html}
            </body>
        </html>"#
    );

    let options = gotenberg_pdf::WebOptions {
        skip_network_idle_events: Some(false),
        ..Default::default()
    };

    let pdf_bytes = client
        .pdf_from_html(&full_html, options)
        .await
        .map_err(|e| anywho::anywho!("{e}"))?;

    tokio::fs::write(file_destination_path, pdf_bytes)
        .await
        .map_err(|e| anywho::anywho!("{e}"))?;

    Ok(())
}
