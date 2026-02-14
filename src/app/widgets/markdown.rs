#![allow(dead_code)]

// This is an adapted version of the official iced markdown widget: https://github.com/iced-rs/iced/blob/master/widget/src/markdown.rs
// it has been modified to work with the current version of libcosmic, original code by @hecrj

//! Markdown widgets can parse and display Markdown.
//!
//! You can enable the `highlighter` feature for syntax highlighting
//! in code blocks.
//!
//! Only the variants of [`Item`] are currently supported.
//!
//! # Example
//! ```no_run
//! # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
//! # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
//! #
//! use iced::widget::markdown;
//! use iced::Theme;
//!
//! struct State {
//!    markdown: Vec<markdown::Item>,
//! }
//!
//! enum Message {
//!     LinkClicked(markdown::Url),
//! }
//!
//! impl State {
//!     pub fn new() -> Self {
//!         Self {
//!             markdown: markdown::parse("This is some **Markdown**!").collect(),
//!         }
//!     }
//!
//!     fn view(&self) -> Element<'_, Message> {
//!         markdown::view(
//!             &self.markdown,
//!             markdown::Settings::default(),
//!         )
//!         .map(Message::LinkClicked)
//!         .into()
//!     }
//!
//!     fn update(state: &mut State, message: Message) {
//!         match message {
//!             Message::LinkClicked(url) => {
//!                 println!("The following url was clicked: {url}");
//!             }
//!         }
//!     }
//! }
//! ```
use cosmic::iced::highlighter;
use cosmic::iced_core::alignment;
use cosmic::iced_core::border;
use cosmic::iced_core::font::{self, Font};
use cosmic::iced_core::padding;
use cosmic::iced_core::theme;
use cosmic::iced_core::{Color, Element, Length, Padding, Pixels, color};
use cosmic::iced_widget::Rule;
use cosmic::iced_widget::{
    checkbox, column, container, rich_text, row, rule, scrollable, span, text,
};
use cosmic::theme::Theme;

use std::borrow::BorrowMut;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::mem;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

pub use cosmic::iced_core::text::Highlight;
pub use pulldown_cmark::HeadingLevel;
pub use url::Url;

/// A bunch of Markdown that has been parsed.
#[derive(Debug, Default)]
pub struct Content {
    items: Vec<Item>,
    incomplete: HashMap<usize, Section>,
    state: State,
}

/// Messages that can be emitted by the markdown viewer
#[derive(Debug, Clone)]
pub enum MarkdownMessage {
    /// A link was clicked
    LinkClicked(Url),
    /// An image became visible and should be loaded
    ImageShown(Url),
}

#[derive(Debug)]
struct Section {
    content: String,
    broken_links: HashSet<String>,
}

impl Content {
    /// Creates a new empty [`Content`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates some new [`Content`] by parsing the given Markdown.
    pub fn parse(markdown: &str) -> Self {
        let mut content = Self::new();
        content.push_str(markdown);
        content
    }

    /// Pushes more Markdown into the [`Content`]; parsing incrementally!
    ///
    /// This is specially useful when you have long streams of Markdown; like
    /// big files or potentially long replies.
    pub fn push_str(&mut self, markdown: &str) {
        if markdown.is_empty() {
            return;
        }

        // Append to last leftover text
        let mut leftover = std::mem::take(&mut self.state.leftover);
        leftover.push_str(markdown);

        let input = if leftover.trim_end().ends_with('|') {
            leftover.trim_end().trim_end_matches('|')
        } else {
            leftover.as_str()
        };

        // Pop the last item
        let _ = self.items.pop();

        // Re-parse last item and new text
        for (item, source, broken_links) in parse_with(&mut self.state, input) {
            if !broken_links.is_empty() {
                let _ = self.incomplete.insert(
                    self.items.len(),
                    Section {
                        content: source.to_owned(),
                        broken_links,
                    },
                );
            }

            self.items.push(item);
        }

        self.state.leftover.push_str(&leftover[input.len()..]);

        // Re-parse incomplete sections if new references are available
        if !self.incomplete.is_empty() {
            self.incomplete.retain(|index, section| {
                if self.items.len() <= *index {
                    return false;
                }

                let broken_links_before = section.broken_links.len();

                section
                    .broken_links
                    .retain(|link| !self.state.references.contains_key(link));

                if broken_links_before != section.broken_links.len() {
                    let mut state = State {
                        leftover: String::new(),
                        references: self.state.references.clone(),
                        images: HashSet::new(),
                        highlighter: None,
                    };

                    if let Some((item, _source, _broken_links)) =
                        parse_with(&mut state, &section.content).next()
                    {
                        self.items[*index] = item;
                    }

                    self.state.images.extend(state.images.drain());
                    drop(state);
                }

                !section.broken_links.is_empty()
            });
        }
    }

    /// Returns the Markdown items, ready to be rendered.
    ///
    /// You can use [`view`] to turn them into an [`Element`].
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Returns the URLs of the Markdown images present in the [`Content`].
    pub fn images(&self) -> &HashSet<Url> {
        &self.state.images
    }
}

/// A Markdown item.
#[derive(Debug, Clone)]
pub enum Item {
    /// A heading.
    Heading(pulldown_cmark::HeadingLevel, Text),
    /// A paragraph.
    Paragraph(Text),
    /// A code block.
    ///
    /// You can enable the `highlighter` feature for syntax highlighting.
    CodeBlock {
        /// The language of the code block, if any.
        language: Option<String>,
        /// The raw code of the code block.
        code: String,
        /// The styled lines of text in the code block.
        lines: Vec<Text>,
    },
    /// A list.
    List {
        /// The first number of the list, if it is ordered.
        start: Option<u64>,
        /// The items of the list.
        bullets: Vec<Bullet>,
    },
    /// An image.
    Image {
        /// The destination URL of the image.
        url: Url,
        /// The title of the image.
        title: String,
        /// The alternative text of the image.
        alt: Text,
    },
    /// A quote.
    Quote(Vec<Item>),
    /// A horizontal separator.
    Rule,
    /// A table.
    Table {
        /// The columns of the table.
        columns: Vec<Column>,
        /// The rows of the table.
        rows: Vec<Row>,
    },
}

/// The column of a table.
#[derive(Debug, Clone)]
pub struct Column {
    /// The header of the column.
    pub header: Vec<Item>,
    /// The alignment of the column.
    pub alignment: pulldown_cmark::Alignment,
}

/// The row of a table.
#[derive(Debug, Clone)]
pub struct Row {
    /// The cells of the row.
    cells: Vec<Vec<Item>>,
}

/// A bunch of parsed Markdown text.
#[derive(Debug, Clone)]
pub struct Text {
    spans: Vec<Span>,
    last_style: Cell<Option<Style>>,
    last_styled_spans: RefCell<Arc<[text::Span<'static, MarkdownMessage>]>>,
}

impl Text {
    fn new(spans: Vec<Span>) -> Self {
        Self {
            spans,
            last_style: Cell::default(),
            last_styled_spans: RefCell::default(),
        }
    }

    /// Returns the [`rich_text()`] spans ready to be used for the given style.
    ///
    /// This method performs caching for you. It will only reallocate if the [`Style`]
    /// provided changes.
    pub fn spans(&self, style: Style) -> Arc<[text::Span<'static, MarkdownMessage>]> {
        if Some(style) != self.last_style.get() {
            *self.last_styled_spans.borrow_mut() =
                self.spans.iter().map(|span| span.view(&style)).collect();

            self.last_style.set(Some(style));
        }

        self.last_styled_spans.borrow().clone()
    }
}

#[derive(Debug, Clone)]
enum Span {
    Standard {
        text: String,
        strikethrough: bool,
        link: Option<Url>,
        strong: bool,
        emphasis: bool,
        code: bool,
    },
    Highlight {
        text: String,
        color: Option<Color>,
        font: Option<Font>,
    },
}

impl Span {
    fn view(&self, style: &Style) -> text::Span<'static, MarkdownMessage> {
        match self {
            Span::Standard {
                text,
                strikethrough,
                link,
                strong,
                emphasis,
                code,
            } => {
                let span = span(text.clone()).strikethrough(*strikethrough);

                let span = if *code {
                    span.font(style.inline_code_font)
                        .color(style.inline_code_color)
                        .background(style.inline_code_highlight.background)
                        .border(style.inline_code_highlight.border)
                        .padding(style.inline_code_padding)
                } else if *strong || *emphasis {
                    span.font(Font {
                        weight: if *strong {
                            font::Weight::Bold
                        } else {
                            font::Weight::Normal
                        },
                        style: if *emphasis {
                            font::Style::Italic
                        } else {
                            font::Style::Normal
                        },
                        ..style.font
                    })
                } else {
                    span.font(style.font)
                };

                if let Some(link) = link.as_ref() {
                    span.color(style.link_color)
                        .link(MarkdownMessage::LinkClicked(link.clone()))
                } else {
                    span
                }
            }
            Span::Highlight { text, color, font } => {
                span(text.clone()).color_maybe(*color).font_maybe(*font)
            }
        }
    }
}

/// The item of a list.
#[derive(Debug, Clone)]
pub enum Bullet {
    /// A simple bullet point.
    Point {
        /// The contents of the bullet point.
        items: Vec<Item>,
    },
    /// A task.
    Task {
        /// The contents of the task.
        items: Vec<Item>,
        /// Whether the task is done or not.
        done: bool,
    },
}

impl Bullet {
    fn items(&self) -> &[Item] {
        match self {
            Bullet::Point { items } | Bullet::Task { items, .. } => items,
        }
    }

    fn push(&mut self, item: Item) {
        let (Bullet::Point { items } | Bullet::Task { items, .. }) = self;

        items.push(item);
    }
}

/// Parse the given Markdown content.
///
/// # Example
/// ```no_run
/// # mod iced { pub mod widget { pub use iced_widget::*; } pub use iced_widget::Renderer; pub use iced_widget::core::*; }
/// # pub type Element<'a, Message> = iced_widget::core::Element<'a, Message, iced_widget::Theme, iced_widget::Renderer>;
/// #
/// use iced::widget::markdown;
/// use iced::Theme;
///
/// struct State {
///    markdown: Vec<markdown::Item>,
/// }
///
/// enum Message {
///     LinkClicked(markdown::Url),
/// }
///
/// impl State {
///     pub fn new() -> Self {
///         Self {
///             markdown: markdown::parse("This is some **Markdown**!").collect(),
///         }
///     }
///
///     fn view(&self) -> Element<'_, Message> {
///         markdown::view(
///             &self.markdown,
///             markdown::Settings::default(),
///         )
///         .map(Message::LinkClicked)
///         .into()
///     }
///
///     fn update(state: &mut State, message: Message) {
///         match message {
///             Message::LinkClicked(url) => {
///                 println!("The following url was clicked: {url}");
///             }
///         }
///     }
/// }
/// ```
pub fn parse(markdown: &str) -> impl Iterator<Item = Item> + '_ {
    parse_with(State::default(), markdown).map(|(item, _source, _broken_links)| item)
}

#[derive(Debug, Default)]
struct State {
    leftover: String,
    references: HashMap<String, String>,
    images: HashSet<Url>,
    highlighter: Option<Highlighter>,
}

#[derive(Debug)]
struct Highlighter {
    lines: Vec<(String, Vec<Span>)>,
    language: String,
    parser: highlighter::Stream,
    current: usize,
}

impl Highlighter {
    pub fn new(language: &str) -> Self {
        Self {
            lines: Vec::new(),
            parser: highlighter::Stream::new(&highlighter::Settings {
                theme: highlighter::Theme::Base16Ocean,
                token: language.to_owned(),
            }),
            language: language.to_owned(),
            current: 0,
        }
    }

    pub fn prepare(&mut self) {
        self.current = 0;
    }

    pub fn highlight_line(&mut self, text: &str) -> &[Span] {
        match self.lines.get(self.current) {
            Some(line) if line.0 == text => {}
            _ => {
                if self.current + 1 < self.lines.len() {
                    self.parser.reset();
                    self.lines.truncate(self.current);

                    for line in &self.lines {
                        let _ = self.parser.highlight_line(&line.0);
                    }
                }

                if self.current + 1 < self.lines.len() {
                    self.parser.commit();
                }

                let mut spans = Vec::new();

                for (range, highlight) in self.parser.highlight_line(text) {
                    spans.push(Span::Highlight {
                        text: text[range].to_owned(),
                        color: highlight.color(),
                        font: highlight.font(),
                    });
                }

                if self.current + 1 == self.lines.len() {
                    let _ = self.lines.pop();
                }

                self.lines.push((text.to_owned(), spans));
            }
        }

        self.current += 1;

        &self
            .lines
            .get(self.current - 1)
            .expect("Line must be parsed")
            .1
    }
}

fn parse_with<'a>(
    mut state: impl BorrowMut<State> + 'a,
    markdown: &'a str,
) -> impl Iterator<Item = (Item, &'a str, HashSet<String>)> + 'a {
    enum Scope {
        List(List),
        Quote(Vec<Item>),
        Table {
            alignment: Vec<pulldown_cmark::Alignment>,
            columns: Vec<Column>,
            rows: Vec<Row>,
            current: Vec<Item>,
        },
    }

    struct List {
        start: Option<u64>,
        bullets: Vec<Bullet>,
    }

    let broken_links = Rc::new(RefCell::new(HashSet::new()));

    let mut spans = Vec::new();
    let mut code = String::new();
    let mut code_language = None;
    let mut code_lines = Vec::new();
    let mut strong = false;
    let mut emphasis = false;
    let mut strikethrough = false;
    let mut metadata = false;
    let mut code_block = false;
    let mut link = None;
    let mut image = None;
    let mut stack = Vec::new();

    let mut highlighter = None;

    let parser = pulldown_cmark::Parser::new_with_broken_link_callback(
        markdown,
        pulldown_cmark::Options::ENABLE_TABLES
            | pulldown_cmark::Options::ENABLE_STRIKETHROUGH
            | pulldown_cmark::Options::ENABLE_TASKLISTS,
        {
            let references = state.borrow().references.clone();
            let broken_links = broken_links.clone();

            Some(move |broken_link: pulldown_cmark::BrokenLink<'_>| {
                if let Some(reference) = references.get(broken_link.reference.as_ref()) {
                    Some((
                        pulldown_cmark::CowStr::from(reference.to_owned()),
                        broken_link.reference.into_static(),
                    ))
                } else {
                    let _ = RefCell::borrow_mut(&broken_links)
                        .insert(broken_link.reference.into_string());

                    None
                }
            })
        },
    );

    let references = &mut state.borrow_mut().references;

    for reference in parser.reference_definitions().iter() {
        let _ = references.insert(reference.0.to_owned(), reference.1.dest.to_string());
    }

    let produce = move |state: &mut State, stack: &mut Vec<Scope>, item, source: Range<usize>| {
        if let Some(scope) = stack.last_mut() {
            match scope {
                Scope::List(list) => {
                    list.bullets.last_mut().expect("item context").push(item);
                }
                Scope::Quote(items) => {
                    items.push(item);
                }
                Scope::Table { current, .. } => {
                    current.push(item);
                }
            }

            None
        } else {
            state.leftover = markdown[source.start..].to_owned();

            Some((
                item,
                &markdown[source.start..source.end],
                broken_links.take(),
            ))
        }
    };

    let parser = parser.into_offset_iter();

    // We want to keep the `spans` capacity
    #[allow(clippy::drain_collect)]
    parser.filter_map(move |(event, source)| match event {
        pulldown_cmark::Event::Start(tag) => match tag {
            pulldown_cmark::Tag::Strong if !metadata => {
                strong = true;
                None
            }
            pulldown_cmark::Tag::Emphasis if !metadata => {
                emphasis = true;
                None
            }
            pulldown_cmark::Tag::Strikethrough if !metadata => {
                strikethrough = true;
                None
            }
            pulldown_cmark::Tag::Link { dest_url, .. } if !metadata => {
                match Url::parse(&dest_url) {
                    Ok(url) if url.scheme() == "http" || url.scheme() == "https" => {
                        link = Some(url);
                    }
                    _ => {}
                }
                None
            }
            pulldown_cmark::Tag::Image {
                dest_url, title, ..
            } if !metadata => {
                match Url::parse(&dest_url) {
                    Ok(url)
                        if url.scheme() == "http"
                            || url.scheme() == "https"
                            || url.scheme() == "file" =>
                    {
                        image = Some((url, title.into_string()));
                    }
                    _ => {}
                }
                None
            }
            pulldown_cmark::Tag::List(first_item) if !metadata => {
                let prev = if spans.is_empty() {
                    None
                } else {
                    produce(
                        state.borrow_mut(),
                        &mut stack,
                        Item::Paragraph(Text::new(spans.drain(..).collect())),
                        source,
                    )
                };

                stack.push(Scope::List(List {
                    start: first_item,
                    bullets: Vec::new(),
                }));

                prev
            }
            pulldown_cmark::Tag::Item => {
                if let Some(Scope::List(list)) = stack.last_mut() {
                    list.bullets.push(Bullet::Point { items: Vec::new() });
                }

                None
            }
            pulldown_cmark::Tag::BlockQuote(_kind) if !metadata => {
                let prev = if spans.is_empty() {
                    None
                } else {
                    produce(
                        state.borrow_mut(),
                        &mut stack,
                        Item::Paragraph(Text::new(spans.drain(..).collect())),
                        source,
                    )
                };

                stack.push(Scope::Quote(Vec::new()));

                prev
            }
            pulldown_cmark::Tag::CodeBlock(kind) if !metadata => {
                let language = match kind {
                    pulldown_cmark::CodeBlockKind::Fenced(lang) => lang,
                    pulldown_cmark::CodeBlockKind::Indented => pulldown_cmark::CowStr::from(""),
                };

                {
                    highlighter = Some({
                        let mut highlighter = state
                            .borrow_mut()
                            .highlighter
                            .take()
                            .filter(|highlighter| highlighter.language == language.as_ref())
                            .unwrap_or_else(|| {
                                Highlighter::new(language.split(',').next().unwrap_or_default())
                            });

                        highlighter.prepare();

                        highlighter
                    });
                }

                code_block = true;
                code_language = (!language.is_empty()).then(|| language.into_string());

                if spans.is_empty() {
                    None
                } else {
                    produce(
                        state.borrow_mut(),
                        &mut stack,
                        Item::Paragraph(Text::new(spans.drain(..).collect())),
                        source,
                    )
                }
            }
            pulldown_cmark::Tag::MetadataBlock(_) => {
                metadata = true;
                None
            }
            pulldown_cmark::Tag::Table(alignment) => {
                stack.push(Scope::Table {
                    columns: Vec::with_capacity(alignment.len()),
                    alignment,
                    current: Vec::new(),
                    rows: Vec::new(),
                });

                None
            }
            pulldown_cmark::Tag::TableHead => {
                strong = true;
                None
            }
            pulldown_cmark::Tag::TableRow => {
                let Scope::Table { rows, .. } = stack.last_mut()? else {
                    return None;
                };

                rows.push(Row { cells: Vec::new() });
                None
            }
            _ => None,
        },
        pulldown_cmark::Event::End(tag) => match tag {
            pulldown_cmark::TagEnd::Heading(level) if !metadata => produce(
                state.borrow_mut(),
                &mut stack,
                Item::Heading(level, Text::new(spans.drain(..).collect())),
                source,
            ),
            pulldown_cmark::TagEnd::Strong if !metadata => {
                strong = false;
                None
            }
            pulldown_cmark::TagEnd::Emphasis if !metadata => {
                emphasis = false;
                None
            }
            pulldown_cmark::TagEnd::Strikethrough if !metadata => {
                strikethrough = false;
                None
            }
            pulldown_cmark::TagEnd::Link if !metadata => {
                link = None;
                None
            }
            pulldown_cmark::TagEnd::Paragraph if !metadata => {
                if spans.is_empty() {
                    None
                } else {
                    produce(
                        state.borrow_mut(),
                        &mut stack,
                        Item::Paragraph(Text::new(spans.drain(..).collect())),
                        source,
                    )
                }
            }
            pulldown_cmark::TagEnd::Item if !metadata => {
                if spans.is_empty() {
                    None
                } else {
                    produce(
                        state.borrow_mut(),
                        &mut stack,
                        Item::Paragraph(Text::new(spans.drain(..).collect())),
                        source,
                    )
                }
            }
            pulldown_cmark::TagEnd::List(_) if !metadata => {
                let scope = stack.pop()?;

                let Scope::List(list) = scope else {
                    return None;
                };

                produce(
                    state.borrow_mut(),
                    &mut stack,
                    Item::List {
                        start: list.start,
                        bullets: list.bullets,
                    },
                    source,
                )
            }
            pulldown_cmark::TagEnd::BlockQuote(_kind) if !metadata => {
                let scope = stack.pop()?;

                let Scope::Quote(quote) = scope else {
                    return None;
                };

                produce(state.borrow_mut(), &mut stack, Item::Quote(quote), source)
            }
            pulldown_cmark::TagEnd::Image if !metadata => {
                let (url, title) = image.take()?;
                let alt = Text::new(spans.drain(..).collect());

                let state = state.borrow_mut();
                let _ = state.images.insert(url.clone());

                produce(state, &mut stack, Item::Image { url, title, alt }, source)
            }
            pulldown_cmark::TagEnd::CodeBlock if !metadata => {
                code_block = false;

                {
                    state.borrow_mut().highlighter = highlighter.take();
                }

                produce(
                    state.borrow_mut(),
                    &mut stack,
                    Item::CodeBlock {
                        language: code_language.take(),
                        code: mem::take(&mut code),
                        lines: code_lines.drain(..).collect(),
                    },
                    source,
                )
            }
            pulldown_cmark::TagEnd::MetadataBlock(_) => {
                metadata = false;
                None
            }
            pulldown_cmark::TagEnd::Table => {
                let scope = stack.pop()?;

                let Scope::Table { columns, rows, .. } = scope else {
                    return None;
                };

                produce(
                    state.borrow_mut(),
                    &mut stack,
                    Item::Table { columns, rows },
                    source,
                )
            }
            pulldown_cmark::TagEnd::TableHead => {
                strong = false;
                None
            }
            pulldown_cmark::TagEnd::TableCell => {
                if !spans.is_empty() {
                    let _ = produce(
                        state.borrow_mut(),
                        &mut stack,
                        Item::Paragraph(Text::new(spans.drain(..).collect())),
                        source,
                    );
                }

                let Scope::Table {
                    alignment,
                    columns,
                    rows,
                    current,
                } = stack.last_mut()?
                else {
                    return None;
                };

                if columns.len() < alignment.len() {
                    columns.push(Column {
                        header: std::mem::take(current),
                        alignment: alignment[columns.len()],
                    });
                } else {
                    rows.last_mut()
                        .expect("table row")
                        .cells
                        .push(std::mem::take(current));
                }

                None
            }
            _ => None,
        },
        pulldown_cmark::Event::Text(text) if !metadata => {
            if code_block {
                code.push_str(&text);

                if let Some(highlighter) = &mut highlighter {
                    for line in text.lines() {
                        code_lines.push(Text::new(highlighter.highlight_line(line).to_vec()));
                    }
                } else {
                    for line in text.lines() {
                        code_lines.push(Text::new(vec![Span::Standard {
                            text: line.to_owned(),
                            strong,
                            emphasis,
                            strikethrough,
                            link: link.clone(),
                            code: false,
                        }]));
                    }
                }

                return None;
            }

            let span = Span::Standard {
                text: text.into_string(),
                strong,
                emphasis,
                strikethrough,
                link: link.clone(),
                code: false,
            };

            spans.push(span);

            None
        }
        pulldown_cmark::Event::Code(code) if !metadata => {
            let span = Span::Standard {
                text: code.into_string(),
                strong,
                emphasis,
                strikethrough,
                link: link.clone(),
                code: true,
            };

            spans.push(span);
            None
        }
        pulldown_cmark::Event::SoftBreak if !metadata => {
            spans.push(Span::Standard {
                text: String::from(" "),
                strikethrough,
                strong,
                emphasis,
                link: link.clone(),
                code: false,
            });
            None
        }
        pulldown_cmark::Event::HardBreak if !metadata => {
            spans.push(Span::Standard {
                text: String::from("\n"),
                strikethrough,
                strong,
                emphasis,
                link: link.clone(),
                code: false,
            });
            None
        }
        pulldown_cmark::Event::Rule => produce(state.borrow_mut(), &mut stack, Item::Rule, source),
        pulldown_cmark::Event::TaskListMarker(done) => {
            if let Some(Scope::List(list)) = stack.last_mut()
                && let Some(item) = list.bullets.last_mut()
                && let Bullet::Point { items } = item
            {
                *item = Bullet::Task {
                    items: std::mem::take(items),
                    done,
                };
            }

            None
        }
        _ => None,
    })
}

/// Configuration controlling Markdown rendering in [`view`].
#[derive(Debug, Clone, Copy)]
pub struct Settings {
    /// The base text size.
    pub text_size: Pixels,
    /// The text size of level 1 heading.
    pub h1_size: Pixels,
    /// The text size of level 2 heading.
    pub h2_size: Pixels,
    /// The text size of level 3 heading.
    pub h3_size: Pixels,
    /// The text size of level 4 heading.
    pub h4_size: Pixels,
    /// The text size of level 5 heading.
    pub h5_size: Pixels,
    /// The text size of level 6 heading.
    pub h6_size: Pixels,
    /// The text size used in code blocks.
    pub code_size: Pixels,
    /// The spacing to be used between elements.
    pub spacing: Pixels,
    /// The styling of the Markdown.
    pub style: Style,
}

impl Settings {
    /// Creates new [`Settings`] with default text size and the given [`Style`].
    pub fn with_style(style: impl Into<Style>) -> Self {
        Self::with_text_size(16, style)
    }

    /// Creates new [`Settings`] with the given base text size in [`Pixels`].
    ///
    /// Heading levels will be adjusted automatically. Specifically,
    /// the first level will be twice the base size, and then every level
    /// after that will be 25% smaller.
    pub fn with_text_size(text_size: impl Into<Pixels>, style: impl Into<Style>) -> Self {
        let text_size = text_size.into();

        Self {
            text_size,
            h1_size: text_size * 2.0,
            h2_size: text_size * 1.75,
            h3_size: text_size * 1.5,
            h4_size: text_size * 1.25,
            h5_size: text_size,
            h6_size: text_size,
            code_size: text_size * 0.75,
            spacing: text_size * 0.875,
            style: style.into(),
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::with_style(Style::default())
    }
}

impl From<&Theme> for Settings {
    fn from(theme: &Theme) -> Self {
        Self::with_style(Style::from(theme))
    }
}

impl From<Theme> for Settings {
    fn from(theme: Theme) -> Self {
        Self::with_style(Style::from(theme))
    }
}

/// The text styling of some Markdown rendering in [`view`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// The [`Font`] to be applied to basic text.
    pub font: Font,
    /// The [`Highlight`] to be applied to the background of inline code.
    pub inline_code_highlight: Highlight,
    /// The [`Padding`] to be applied to the background of inline code.
    pub inline_code_padding: Padding,
    /// The [`Color`] to be applied to inline code.
    pub inline_code_color: Color,
    /// The [`Font`] to be applied to inline code.
    pub inline_code_font: Font,
    /// The [`Font`] to be applied to code blocks.
    pub code_block_font: Font,
    /// The [`Color`] to be applied to links.
    pub link_color: Color,
}

impl Style {
    /// Creates a new [`Style`] from the given [`theme::Palette`].
    pub fn from_palette(palette: theme::Palette) -> Self {
        Self {
            font: Font::default(),
            inline_code_padding: padding::left(1).right(1),
            inline_code_highlight: Highlight {
                background: color!(0x111111).into(),
                border: border::rounded(4),
            },
            inline_code_color: Color::WHITE,
            inline_code_font: Font::MONOSPACE,
            code_block_font: Font::MONOSPACE,
            link_color: palette.primary,
        }
    }
}

impl Default for Style {
    fn default() -> Self {
        Self {
            font: Font::default(),
            inline_code_padding: padding::left(1).right(1),
            inline_code_highlight: Highlight {
                background: color!(0x111111).into(),
                border: border::rounded(4),
            },
            inline_code_color: Color::WHITE,
            inline_code_font: Font::MONOSPACE,
            code_block_font: Font::MONOSPACE,
            link_color: color!(0x6495ED),
        }
    }
}

impl From<theme::Palette> for Style {
    fn from(palette: theme::Palette) -> Self {
        Self::from_palette(palette)
    }
}

impl From<&Theme> for Style {
    fn from(_theme: &Theme) -> Self {
        // Cosmic Theme doesn't have a simple palette() method
        // Use default style instead
        Self::default()
    }
}

impl From<Theme> for Style {
    fn from(_theme: Theme) -> Self {
        // Cosmic Theme doesn't have a simple palette() method
        // Use default style instead
        Self::default()
    }
}

/// Display a bunch of Markdown items.
///
/// You can obtain the items with [`parse`].
pub fn view<'a, Theme, Renderer>(
    items: impl IntoIterator<Item = &'a Item>,
    settings: impl Into<Settings>,
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    view_with(items, settings, &DefaultViewer)
}

/// Runs [`view`] but with a custom [`Viewer`] to turn an [`Item`] into
/// an [`Element`].
///
/// This is useful if you want to customize the look of certain Markdown
/// elements.
pub fn view_with<'a, Theme, Renderer>(
    items: impl IntoIterator<Item = &'a Item>,
    settings: impl Into<Settings>,
    viewer: &impl Viewer<'a, Theme, Renderer>,
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    let settings = settings.into();

    let blocks = items
        .into_iter()
        .enumerate()
        .map(|(i, item_)| item(viewer, settings, item_, i));

    Element::new(column(blocks).spacing(settings.spacing))
}

/// Displays an [`Item`] using the given [`Viewer`].
pub fn item<'a, Theme, Renderer>(
    viewer: &impl Viewer<'a, Theme, Renderer>,
    settings: Settings,
    item: &'a Item,
    index: usize,
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    match item {
        Item::Image { url, title, alt } => viewer.image(settings, url, title, alt),
        Item::Heading(level, text) => viewer.heading(settings, level, text, index),
        Item::Paragraph(text) => viewer.paragraph(settings, text),
        Item::CodeBlock {
            language,
            code,
            lines,
        } => viewer.code_block(settings, language.as_deref(), code, lines),
        Item::List {
            start: None,
            bullets,
        } => viewer.unordered_list(settings, bullets),
        Item::List {
            start: Some(start),
            bullets,
        } => viewer.ordered_list(settings, *start, bullets),
        Item::Quote(quote) => viewer.quote(settings, quote),
        Item::Rule => viewer.rule(settings),
        Item::Table { columns, rows } => viewer.table(settings, columns, rows),
    }
}

/// Displays a heading using the default look.
pub fn heading<'a, Theme, Renderer>(
    settings: Settings,
    level: &'a HeadingLevel,
    text: &'a Text,
    index: usize,
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    let Settings {
        h1_size,
        h2_size,
        h3_size,
        h4_size,
        h5_size,
        h6_size,
        text_size,
        ..
    } = settings;

    container(rich_text(text.spans(settings.style)).size(match level {
        pulldown_cmark::HeadingLevel::H1 => h1_size,
        pulldown_cmark::HeadingLevel::H2 => h2_size,
        pulldown_cmark::HeadingLevel::H3 => h3_size,
        pulldown_cmark::HeadingLevel::H4 => h4_size,
        pulldown_cmark::HeadingLevel::H5 => h5_size,
        pulldown_cmark::HeadingLevel::H6 => h6_size,
    }))
    .padding(padding::top(if index > 0 {
        text_size / 2.0
    } else {
        Pixels::ZERO
    }))
    .into()
}

/// Displays a paragraph using the default look.
pub fn paragraph<'a, Theme, Renderer>(
    settings: Settings,
    text: &Text,
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    rich_text(text.spans(settings.style))
        .size(settings.text_size)
        .into()
}

/// Displays an unordered list using the default look and
/// calling the [`Viewer`] for each bullet point item.
pub fn unordered_list<'a, Theme, Renderer>(
    viewer: &impl Viewer<'a, Theme, Renderer>,
    settings: Settings,
    bullets: &'a [Bullet],
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    column(bullets.iter().map(|bullet| {
        row![
            match bullet {
                Bullet::Point { .. } => {
                    text("•").size(settings.text_size).into()
                }
                Bullet::Task { done, .. } => {
                    Element::from(
                        container(checkbox("", *done).size(settings.text_size))
                            .center_y(text::LineHeight::default().to_absolute(settings.text_size)),
                    )
                }
            },
            view_with(
                bullet.items(),
                Settings {
                    spacing: settings.spacing * 0.6,
                    ..settings
                },
                viewer,
            )
        ]
        .spacing(settings.spacing)
        .into()
    }))
    .spacing(settings.spacing * 0.75)
    .padding([0.0, settings.spacing.0])
    .into()
}

/// Displays an ordered list using the default look and
/// calling the [`Viewer`] for each numbered item.
pub fn ordered_list<'a, Theme, Renderer>(
    viewer: &impl Viewer<'a, Theme, Renderer>,
    settings: Settings,
    start: u64,
    bullets: &'a [Bullet],
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    let digits = ((start + bullets.len() as u64).max(1) as f32)
        .log10()
        .ceil();

    column(bullets.iter().enumerate().map(|(i, bullet)| {
        row![
            text!("{}.", i as u64 + start)
                .size(settings.text_size)
                .align_x(alignment::Horizontal::Right)
                .width(settings.text_size * ((digits / 2.0).ceil() + 1.0)),
            view_with(
                bullet.items(),
                Settings {
                    spacing: settings.spacing * 0.6,
                    ..settings
                },
                viewer,
            )
        ]
        .spacing(settings.spacing)
        .into()
    }))
    .spacing(settings.spacing * 0.75)
    .into()
}

/// Displays a code block using the default look.
pub fn code_block<'a, Theme, Renderer>(
    settings: Settings,
    lines: &'a [Text],
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    container(
        scrollable(
            container(column(lines.iter().map(|line| {
                rich_text(line.spans(settings.style))
                    .font(settings.style.code_block_font)
                    .size(settings.code_size)
                    .into()
            })))
            .padding(settings.code_size.0),
        )
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::default()
                .width(settings.code_size / 2.0)
                .scroller_width(settings.code_size / 2.0),
        )),
    )
    .width(Length::Fill)
    .padding(settings.code_size.0 / 4.0)
    .class(Theme::code_block())
    .into()
}

/// Displays a quote using the default look.
pub fn quote<'a, Theme, Renderer>(
    viewer: &impl Viewer<'a, Theme, Renderer>,
    settings: Settings,
    contents: &'a [Item],
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    row![
        container(cosmic::iced_widget::vertical_rule(4.0))
            .width(4.0)
            .height(Length::Fill),
        column(
            contents
                .iter()
                .enumerate()
                .map(|(i, content)| item(viewer, settings, content, i)),
        )
        .spacing(settings.spacing.0),
    ]
    .height(Length::Shrink)
    .align_y(cosmic::iced::Alignment::Center)
    .spacing(settings.spacing.0)
    .into()
}

/// Displays a rule using the default look.
pub fn rule<'a, Theme, Renderer>() -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    Rule::horizontal(2).into()
}

/// Displays a table using the default look.
pub fn table<'a, Theme, Renderer>(
    viewer: &impl Viewer<'a, Theme, Renderer>,
    settings: Settings,
    columns: &'a [Column],
    rows: &'a [Row],
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    // Simple table implementation using rows and columns
    let header_row = row(columns.iter().map(|column| {
        container(items(viewer, settings, &column.header))
            .padding(settings.spacing.0 / 2.0)
            .width(Length::Fill)
            .into()
    }))
    .spacing(1.0);

    let body_rows = rows.iter().map(|table_row| {
        row(table_row.cells.iter().enumerate().map(|(i, cells)| {
            let aligned = items(viewer, settings, cells);

            let alignment = columns
                .get(i)
                .map(|c| c.alignment)
                .unwrap_or(pulldown_cmark::Alignment::None);

            container(aligned)
                .padding(settings.spacing.0 / 2.0)
                .width(Length::Fill)
                .align_x(match alignment {
                    pulldown_cmark::Alignment::None | pulldown_cmark::Alignment::Left => {
                        alignment::Horizontal::Left
                    }
                    pulldown_cmark::Alignment::Center => alignment::Horizontal::Center,
                    pulldown_cmark::Alignment::Right => alignment::Horizontal::Right,
                })
                .into()
        }))
        .spacing(1.0)
        .into()
    });

    container(
        column![
            container(header_row)
                .class(Theme::table_header())
                .width(Length::Fill),
            column(body_rows).spacing(1.0)
        ]
        .spacing(1.0),
    )
    .class(Theme::table())
    .width(Length::Fill)
    .into()
}

/// Displays a column of items with the default look.
pub fn items<'a, Theme, Renderer>(
    viewer: &impl Viewer<'a, Theme, Renderer>,
    settings: Settings,
    items: &'a [Item],
) -> Element<'a, MarkdownMessage, Theme, Renderer>
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    column(
        items
            .iter()
            .enumerate()
            .map(|(i, content)| item(viewer, settings, content, i)),
    )
    .spacing(settings.spacing.0)
    .into()
}

/// A view strategy to display a Markdown [`Item`].
pub trait Viewer<'a, Theme = cosmic::theme::Theme, Renderer = cosmic::iced_widget::Renderer>
where
    Self: Sized + 'a,
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
    /// Displays an image.
    ///
    /// By default, it will show a container with the image title.
    fn image(
        &self,
        settings: Settings,
        url: &'a Url,
        title: &'a str,
        alt: &Text,
    ) -> Element<'a, MarkdownMessage, Theme, Renderer> {
        let _url = url;
        let _title = title;

        container(rich_text(alt.spans(settings.style)))
            .padding(settings.spacing.0)
            .class(Theme::code_block())
            .into()
    }

    /// Displays a heading.
    ///
    /// By default, it calls [`heading`].
    fn heading(
        &self,
        settings: Settings,
        level: &'a HeadingLevel,
        text: &'a Text,
        index: usize,
    ) -> Element<'a, MarkdownMessage, Theme, Renderer> {
        heading(settings, level, text, index)
    }

    /// Displays a paragraph.
    ///
    /// By default, it calls [`paragraph`].
    fn paragraph(
        &self,
        settings: Settings,
        text: &Text,
    ) -> Element<'a, MarkdownMessage, Theme, Renderer> {
        paragraph(settings, text)
    }

    /// Displays a code block.
    ///
    /// By default, it calls [`code_block`].
    fn code_block(
        &self,
        settings: Settings,
        language: Option<&'a str>,
        code: &'a str,
        lines: &'a [Text],
    ) -> Element<'a, MarkdownMessage, Theme, Renderer> {
        let _language = language;
        let _code = code;

        code_block(settings, lines)
    }

    /// Displays an unordered list.
    ///
    /// By default, it calls [`unordered_list`].
    fn unordered_list(
        &self,
        settings: Settings,
        bullets: &'a [Bullet],
    ) -> Element<'a, MarkdownMessage, Theme, Renderer> {
        unordered_list(self, settings, bullets)
    }

    /// Displays an ordered list.
    ///
    /// By default, it calls [`ordered_list`].
    fn ordered_list(
        &self,
        settings: Settings,
        start: u64,
        bullets: &'a [Bullet],
    ) -> Element<'a, MarkdownMessage, Theme, Renderer> {
        ordered_list(self, settings, start, bullets)
    }

    /// Displays a quote.
    ///
    /// By default, it calls [`quote`].
    fn quote(
        &self,
        settings: Settings,
        contents: &'a [Item],
    ) -> Element<'a, MarkdownMessage, Theme, Renderer> {
        quote(self, settings, contents)
    }

    /// Displays a rule.
    ///
    /// By default, it calls [`rule`](self::rule()).
    fn rule(&self, _settings: Settings) -> Element<'a, MarkdownMessage, Theme, Renderer> {
        rule::<Theme, Renderer>()
    }

    /// Displays a table.
    ///
    /// By default, it calls [`table`].
    fn table(
        &self,
        settings: Settings,
        columns: &'a [Column],
        rows: &'a [Row],
    ) -> Element<'a, MarkdownMessage, Theme, Renderer> {
        table(self, settings, columns, rows)
    }
}

#[derive(Debug, Clone, Copy)]
struct DefaultViewer;

impl<'a, Theme, Renderer> Viewer<'a, Theme, Renderer> for DefaultViewer
where
    Theme: Catalog + 'a,
    Renderer: cosmic::iced_core::text::Renderer<Font = Font> + 'a,
{
}

/// The theme catalog of Markdown items.
pub trait Catalog:
    container::Catalog + scrollable::Catalog + text::Catalog + rule::Catalog + checkbox::Catalog
{
    /// The styling class of a Markdown code block.
    fn code_block<'a>() -> <Self as container::Catalog>::Class<'a>;

    /// The styling class of a table.
    fn table<'a>() -> <Self as container::Catalog>::Class<'a>;

    /// The styling class of a table header.
    fn table_header<'a>() -> <Self as container::Catalog>::Class<'a>;
}

impl Catalog for Theme {
    fn code_block<'a>() -> <Self as container::Catalog>::Class<'a> {
        cosmic::theme::Container::Secondary
    }

    fn table<'a>() -> <Self as container::Catalog>::Class<'a> {
        cosmic::theme::Container::default()
    }

    fn table_header<'a>() -> <Self as container::Catalog>::Class<'a> {
        cosmic::theme::Container::Secondary
    }
}
