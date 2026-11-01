// SPDX-License-Identifier: GPL-3.0

use std::borrow::Cow;

use cosmic::iced::clipboard::mime::AllowedMimeTypes;

/// Image mime types we accept, in order of preference, with the extension we save them as.
const IMAGE_MIME_TYPES: &[(&str, &str)] = &[
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/webp", "webp"),
    ("image/gif", "gif"),
    ("image/avif", "avif"),
    ("image/bmp", "bmp"),
    ("image/svg+xml", "svg"),
];

/// File lists offered by file managers.
const URI_LIST_MIME_TYPES: &[&str] = &["text/uri-list", "x-special/gnome-copied-files"];

/// Plain text mime types, in order of preference.
const TEXT_MIME_TYPES: &[&str] = &[
    "text/plain;charset=utf-8",
    "text/plain;charset=UTF-8",
    "UTF8_STRING",
    "STRING",
    "TEXT",
    "text/plain",
];

/// What a paste request is after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMode {
    /// Ctrl+V: images become Markdown image tags, anything else is pasted as text.
    Smart,
    /// Paste image action: only images (or copied image files / URLs) are accepted.
    ImageOnly,
}

/// The compositor returns the first mime type of `allowed()` that the source offers,
/// so images win over file lists, and file lists win over plain text.
#[derive(Debug, Clone)]
pub enum ClipboardContent {
    Image {
        bytes: Vec<u8>,
        extension: &'static str,
    },
    Text(String),
}

impl AllowedMimeTypes for ClipboardContent {
    fn allowed() -> Cow<'static, [String]> {
        Cow::Owned(
            IMAGE_MIME_TYPES
                .iter()
                .map(|(mime, _)| *mime)
                .chain(URI_LIST_MIME_TYPES.iter().copied())
                .chain(TEXT_MIME_TYPES.iter().copied())
                .map(String::from)
                .collect(),
        )
    }
}

impl TryFrom<(Vec<u8>, String)> for ClipboardContent {
    type Error = ();

    fn try_from((bytes, mime): (Vec<u8>, String)) -> Result<Self, Self::Error> {
        if let Some((_, extension)) = IMAGE_MIME_TYPES.iter().find(|(m, _)| *m == mime) {
            if bytes.is_empty() {
                return Err(());
            }
            return Ok(Self::Image { bytes, extension });
        }

        let text = String::from_utf8(bytes).map_err(|_| ())?;

        if URI_LIST_MIME_TYPES.contains(&mime.as_str()) {
            // CRLF, `#` comments, and GNOME's leading `copy`/`cut` line
            let list = text
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#') && *l != "copy" && *l != "cut")
                .collect::<Vec<_>>()
                .join("\n");
            return Ok(Self::Text(list));
        }

        if TEXT_MIME_TYPES.contains(&mime.as_str()) {
            return Ok(Self::Text(text.replace("\r\n", "\n")));
        }

        Err(())
    }
}
