use std::collections::HashMap;
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source};
use typst::text::{Font, FontBook};
use typst::{Library, LibraryExt, World};

// Help from: https://github.com/tfachmann/typst-as-library/blob/main/src/lib.rs

pub struct MinimalWorld {
    library: typst::utils::LazyHash<Library>,
    book: typst::utils::LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: Source,
    files: HashMap<FileId, Bytes>,
}

impl MinimalWorld {
    pub fn new(source: &str) -> Self {
        let mut fonts = Vec::new();
        let mut book = FontBook::new();

        for font_data in typst_assets::fonts() {
            let font_bytes = Bytes::new(font_data);
            for font in Font::iter(font_bytes) {
                book.push(font.info().clone());
                fonts.push(font);
            }
        }

        Self {
            library: typst::utils::LazyHash::new(Library::builder().build()),
            book: typst::utils::LazyHash::new(book),
            fonts,
            source: Source::detached(source),
            files: HashMap::new(),
        }
    }
}

impl World for MinimalWorld {
    fn library(&self) -> &typst::utils::LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &typst::utils::LazyHash<FontBook> {
        &self.book
    }
    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> typst::diag::FileResult<Source> {
        if id == self.source.id() {
            Ok(self.source.clone())
        } else {
            Err(typst::diag::FileError::NotFound("<embedded>".into()))
        }
    }

    fn file(&self, id: FileId) -> typst::diag::FileResult<Bytes> {
        self.files
            .get(&id)
            .cloned()
            .ok_or(typst::diag::FileError::NotFound("<embedded>".into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}
