use std::collections::{HashMap, HashSet};

use cosmic::widget::{image, svg};
use frostmark::MarkState;

pub struct MarkdownPreview {
    /// Markdown Preview state
    pub markstate: MarkState,
    /// Images in the Markdown preview
    pub images: HashMap<String, image::Handle>,
    /// SVGs in the Markdown preview
    pub svgs: HashMap<String, svg::Handle>,
    /// Keep track of images in progress/downloading
    pub images_in_progress: HashSet<String>,
}

impl MarkdownPreview {
    pub fn update_content(&mut self, text: &str) {
        self.markstate = MarkState::with_html_and_markdown(text);
    }

    pub fn insert_image(&mut self, url: String, bytes: Vec<u8>) {
        self.images.insert(url, image::Handle::from_bytes(bytes));
    }
    pub fn insert_svg(&mut self, url: String, bytes: Vec<u8>) {
        self.svgs.insert(url, svg::Handle::from_memory(bytes));
    }
}
