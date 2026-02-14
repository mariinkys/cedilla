// SPDX-License-Identifier: GPL-3.0

use crate::app::app_menu::MenuAction;
use crate::app::context_page::ContextPage;
use crate::app::core::utils::{self, CedillaToast};
use crate::app::widgets::{markdown, sensor};
use crate::config::{AppTheme, CedillaConfig};
use crate::fl;
use cosmic::app::context_drawer;
use cosmic::iced::{Alignment, Length, Subscription, highlighter};
use cosmic::iced_widget::{center, column, row};
use cosmic::widget::{self, about::About, menu};
use cosmic::widget::{
    Space, ToastId, Toasts, container, pane_grid, scrollable, text, text_editor, toaster,
};
use cosmic::{prelude::*, surface, theme};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

mod app_menu;
mod context_page;
mod core;
mod widgets;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application toasts
    toasts: Toasts<Message>,
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// The about page for this app.
    about: About,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Application configuration handler
    config_handler: Option<cosmic::cosmic_config::Config>,
    /// Configuration data that persists between application runs.
    config: CedillaConfig,
    // Application Themes
    app_themes: Vec<String>,
    /// Application State
    state: State,
}

/// Represents the Application State
enum State {
    Loading,
    Ready {
        /// Current if/any file path
        path: Option<PathBuf>,
        /// Text Editor Content
        editor_content: text_editor::Content,
        /// Images in the markdown
        markdown_images: HashMap<markdown::Url, ImageState>,
        /// Markdown preview items
        items: Vec<markdown::Item>,
        /// Track if any changes have been made to the current file
        is_dirty: bool,
        /// Pane grid state
        panes: pane_grid::State<PaneContent>,
    },
}

/// Content type for each pane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneContent {
    Editor,
    Preview,
}

/// State of the images in the markdown file
enum ImageState {
    Loading,
    Ready(widget::image::Handle),
    Failed,
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    /// Callback for closing a toast
    CloseToast(ToastId),
    /// Ask to add new toast
    AddToast(CedillaToast),
    /// Opens the given URL in the browser
    LaunchUrl(String),
    /// Opens (or closes if already open) the given [`ContextPage`]
    ToggleContextPage(ContextPage),
    /// Update the application config
    UpdateConfig(CedillaConfig),
    /// Update the application theme
    UpdateTheme(usize),
    /// Callback after clicking something in the app menu
    MenuAction(app_menu::MenuAction),
    /// Needed for responsive menu bar
    Surface(surface::Action),

    /// Creates a new empty file
    NewFile,
    /// Save the current file
    SaveFile,
    /// Callback after opening a new file
    OpenFile(Result<(PathBuf, Arc<String>), anywho::Error>),
    /// Callback after some action is performed on the text editor
    Edit(text_editor::Action),
    /// Callback after saving the current file
    FileSaved(Result<PathBuf, anywho::Error>),

    /// Pane grid resized callback
    PaneResized(pane_grid::ResizeEvent),
    /// Pane grid dragged callback
    PaneDragged(pane_grid::DragEvent),
    /// Triggered when an image becomes visible
    ImageShown(markdown::Url),
    /// Callback after downloading/loading an image
    ImageLoaded(markdown::Url, Result<widget::image::Handle, String>),
}

struct MarkdownViewer<'a> {
    images: &'a HashMap<markdown::Url, ImageState>,
}

impl<'a> markdown::Viewer<'a, cosmic::theme::Theme, cosmic::iced_widget::Renderer>
    for MarkdownViewer<'a>
{
    fn image(
        &self,
        _settings: markdown::Settings,
        url: &'a markdown::Url,
        _title: &'a str,
        _alt: &markdown::Text,
    ) -> Element<'a, markdown::MarkdownMessage> {
        match self.images.get(url) {
            Some(ImageState::Ready(handle)) => {
                // Display the loaded image
                widget::image(handle.clone())
                    .content_fit(cosmic::iced::ContentFit::Contain)
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .into()
            }
            Some(ImageState::Loading) => {
                // Show loading placeholder
                text("Loading image...").into()
            }
            Some(ImageState::Failed) => {
                // Show error state
                text("Failed to load image").into()
            }
            None => sensor(text("Loading..."))
                .key_ref(url.as_str())
                .delay(Duration::from_millis(500))
                .on_show(|_size| markdown::MarkdownMessage::ImageShown(url.clone()))
                .into(),
        }
    }
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = crate::flags::Flags;

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "dev.mariinkys.Cedilla";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    /// Initializes the application with any given flags and startup commands.
    fn init(core: cosmic::Core, flags: Self::Flags) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Create the about widget
        let about = About::default()
            .name("Cedilla")
            .icon(widget::icon::from_name(Self::APP_ID))
            .version(env!("CARGO_PKG_VERSION"))
            .links([
                (fl!("repository"), REPOSITORY),
                (fl!("support"), &format!("{}/issues", REPOSITORY)),
            ])
            .license(env!("CARGO_PKG_LICENSE"))
            .author("mariinkys")
            .developers([("mariinkys", "kysdev.owjga@aleeas.com")])
            .comments("\"Pop Icons\" by System76 is licensed under CC-SA-4.0");

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            toasts: Toasts::new(Message::CloseToast),
            core,
            context_page: ContextPage::default(),
            about,
            key_binds: HashMap::new(),
            config_handler: flags.config_handler,
            config: flags.config,
            app_themes: vec![fl!("match-desktop"), fl!("dark"), fl!("light")],
            state: State::Loading,
        };

        // Startup tasks.
        let tasks = vec![
            app.update_title(),
            cosmic::command::set_theme(app.config.app_theme.theme()),
            Task::done(cosmic::action::app(Message::NewFile)),
        ];

        (app, Task::batch(tasks))
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<'_, Self::Message>> {
        vec![app_menu::menu_bar(&self.core, &self.key_binds)]
    }

    /// Display a context drawer if the context page is requested.
    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Self::Message>> {
        if !self.core.window.show_context {
            return None;
        }

        self.context_page.display(self)
    }

    /// Describes the interface based on the current state of the application model.
    ///
    /// Application events will be processed through the view. Any messages emitted by
    /// events received by widgets will be passed to the update method.
    fn view(&self) -> Element<'_, Self::Message> {
        let content: Element<_> = match &self.state {
            State::Loading => center(text(fl!("loading"))).into(),
            State::Ready {
                path,
                editor_content,
                markdown_images,
                items,
                is_dirty,
                panes,
            } => cedilla_main_view(
                path,
                editor_content,
                markdown_images,
                items,
                is_dirty,
                panes,
            ),
        };

        toaster(&self.toasts, container(content).center(Length::Fill))
    }

    /// Register subscriptions for this application.
    ///
    /// Subscriptions are long-running async tasks running in the background which
    /// emit messages to the application through a channel. They can be dynamically
    /// stopped and started conditionally based on application state, or persist
    /// indefinitely.
    fn subscription(&self) -> Subscription<Self::Message> {
        // Add subscriptions which are always active.
        let subscriptions = vec![
            // Watch for application configuration changes.
            self.core()
                .watch_config::<CedillaConfig>(Self::APP_ID)
                .map(|update| {
                    // for why in update.errors {
                    //     tracing::error!(?why, "app config error");
                    // }

                    Message::UpdateConfig(update.config)
                }),
        ];

        Subscription::batch(subscriptions)
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::CloseToast(id) => {
                self.toasts.remove(id);
                Task::none()
            }
            Message::AddToast(toast) => self.toasts.push(toast.into()).map(cosmic::action::app),
            Message::ToggleContextPage(context_page) => {
                if self.context_page == context_page {
                    // Close the context drawer if the toggled context page is the same.
                    self.core.window.show_context = !self.core.window.show_context;
                } else {
                    // Open the context drawer to display the requested context page.
                    self.context_page = context_page;
                    self.core.window.show_context = true;
                }
                Task::none()
            }
            Message::UpdateConfig(config) => {
                self.config = config;
                Task::none()
            }
            Message::UpdateTheme(index) => {
                let app_theme = match index {
                    1 => AppTheme::Dark,
                    2 => AppTheme::Light,
                    _ => AppTheme::System,
                };

                if let Some(handler) = &self.config_handler {
                    if let Err(err) = self.config.set_app_theme(handler, app_theme) {
                        eprintln!("{err}");
                        // even if it fails we update the config (it won't get saved after restart)
                        let mut old_config = self.config.clone();
                        old_config.app_theme = app_theme;
                        self.config = old_config;
                    }

                    return cosmic::command::set_theme(self.config.app_theme.theme());
                }
                Task::none()
            }
            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => Task::none(),
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                    Task::none()
                }
            },
            Message::MenuAction(action) => {
                let State::Ready { .. } = &mut self.state else {
                    return Task::none();
                };

                match action {
                    MenuAction::About => {
                        self.update(Message::ToggleContextPage(ContextPage::About))
                    }
                    MenuAction::Settings => {
                        self.update(Message::ToggleContextPage(ContextPage::Settings))
                    }
                    MenuAction::OpenFile => Task::perform(
                        async move {
                            match utils::files::open_markdown_file_picker().await {
                                Some(path) => Some(utils::files::load_file(path.into()).await),
                                None => None,
                            }
                        },
                        |res| match res {
                            Some(result) => cosmic::action::app(Message::OpenFile(result)),
                            None => cosmic::action::none(),
                        },
                    ),
                    MenuAction::NewFile => self.update(Message::NewFile),
                    MenuAction::SaveFile => self.update(Message::SaveFile),
                }
            }
            Message::Surface(a) => {
                cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::Surface(a)))
            }

            Message::NewFile => {
                // Create initial pane configuration with editor on left, preview on right
                let (mut panes, first_pane) = pane_grid::State::new(PaneContent::Editor);
                panes.split(pane_grid::Axis::Vertical, first_pane, PaneContent::Preview);

                self.state = State::Ready {
                    path: None,
                    editor_content: text_editor::Content::new(),
                    markdown_images: HashMap::new(),
                    items: vec![],
                    is_dirty: true,
                    panes,
                };
                Task::none()
            }
            Message::SaveFile => {
                let State::Ready {
                    editor_content,
                    path,
                    is_dirty,
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                if !*is_dirty {
                    return Task::none();
                }

                let content = editor_content.text();
                let path = path.clone();

                Task::perform(
                    async move {
                        match path {
                            // We're editing an alreaday existing file
                            Some(path) => Some(utils::files::save_file(path, content).await),
                            // We want to save a new file
                            None => match utils::files::open_markdown_file_saver().await {
                                Some(path) => {
                                    Some(utils::files::save_file(path.into(), content).await)
                                }
                                // Error selecting where to save the file
                                None => None,
                            },
                        }
                    },
                    |res| match res {
                        Some(result) => cosmic::action::app(Message::FileSaved(result)),
                        None => cosmic::action::none(),
                    },
                )
            }
            Message::OpenFile(result) => match result {
                Ok((path, content)) => {
                    // Create initial pane configuration (TODO: Move this to a helper func (also used in NewFile))
                    let (mut panes, first_pane) = pane_grid::State::new(PaneContent::Editor);
                    panes.split(pane_grid::Axis::Vertical, first_pane, PaneContent::Preview);

                    self.state = State::Ready {
                        path: Some(path),
                        editor_content: text_editor::Content::with_text(content.as_ref()),
                        markdown_images: HashMap::new(),
                        items: markdown::parse(content.as_ref()).collect(),
                        is_dirty: false,
                        panes,
                    };
                    Task::none()
                }
                Err(e) => self.update(Message::AddToast(CedillaToast::new(e))),
            },
            Message::Edit(action) => {
                let State::Ready {
                    editor_content,
                    is_dirty,
                    items,
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                *is_dirty = *is_dirty || action.is_edit();
                editor_content.perform(action);
                *items = markdown::parse(editor_content.text().as_ref()).collect();

                Task::none()
            }
            Message::FileSaved(result) => match result {
                Ok(new_path) => {
                    let State::Ready { path, is_dirty, .. } = &mut self.state else {
                        return Task::none();
                    };

                    *path = Some(new_path);
                    *is_dirty = false;

                    self.update(Message::AddToast(CedillaToast::new("File Saved!")))
                }
                Err(e) => self.update(Message::AddToast(CedillaToast::new(e))),
            },

            Message::PaneResized(event) => {
                let State::Ready { panes, .. } = &mut self.state else {
                    return Task::none();
                };

                panes.resize(event.split, event.ratio);
                Task::none()
            }
            Message::PaneDragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                let State::Ready { panes, .. } = &mut self.state else {
                    return Task::none();
                };

                panes.drop(pane, target);
                Task::none()
            }
            Message::PaneDragged(_) => Task::none(),

            Message::ImageShown(url) => {
                let State::Ready {
                    markdown_images, ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                if markdown_images.contains_key(&url) {
                    return Task::none();
                }

                markdown_images.insert(url.clone(), ImageState::Loading);

                let url_for_closure = url.clone();
                Task::perform(utils::images::load_image(url), move |result| {
                    cosmic::action::app(Message::ImageLoaded(url_for_closure.clone(), result))
                })
            }
            Message::ImageLoaded(url, result) => {
                let State::Ready {
                    markdown_images, ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                let state = match result {
                    Ok(handle) => ImageState::Ready(handle),
                    Err(_) => ImageState::Failed,
                };

                markdown_images.insert(url, state);

                Task::none()
            }
        }
    }
}

impl AppModel {
    /// Updates the header and window titles.
    pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
        let window_title = String::from("Cedilla");

        if let Some(id) = self.core.main_window_id() {
            self.set_window_title(window_title, id)
        } else {
            Task::none()
        }
    }

    /// Settings context page
    pub fn settings(&self) -> Element<'_, Message> {
        let app_theme_selected = match self.config.app_theme {
            AppTheme::Dark => 1,
            AppTheme::Light => 2,
            AppTheme::System => 0,
        };

        widget::settings::view_column(vec![
            widget::settings::section()
                .title(fl!("appearance"))
                .add(
                    widget::settings::item::builder(fl!("theme")).control(widget::dropdown(
                        &self.app_themes,
                        Some(app_theme_selected),
                        Message::UpdateTheme,
                    )),
                )
                .into(),
        ])
        .into()
    }
}

//
// VIEWS
//

/// View of the header of this screen
fn cedilla_main_view<'a>(
    path: &'a Option<PathBuf>,
    editor_content: &'a text_editor::Content,
    markdown_images: &'a HashMap<url::Url, ImageState>,
    items: &'a [markdown::Item],
    _is_dirty: &'a bool,
    panes: &'a pane_grid::State<PaneContent>,
) -> Element<'a, Message> {
    let spacing = theme::active().cosmic().spacing;

    let pane_grid = pane_grid::PaneGrid::new(panes, |_pane, content, _is_focused| {
        let (title, icon_name) = match content {
            PaneContent::Editor => (fl!("editor"), "text-editor-symbolic"),
            PaneContent::Preview => (fl!("preview"), "view-paged-symbolic"),
        };

        let title_content = row![
            widget::icon::from_name(icon_name).size(16),
            text(title).size(14),
        ]
        .spacing(spacing.space_xxs)
        .padding([5, spacing.space_xxs])
        .align_y(Alignment::Center);

        let pane_content: Element<'a, Message> = match content {
            PaneContent::Editor => container(
                text_editor(editor_content)
                    .highlight_with::<highlighter::Highlighter>(
                        highlighter::Settings {
                            theme: highlighter::Theme::InspiredGitHub,
                            token: path
                                .as_ref()
                                .and_then(|path| path.extension()?.to_str())
                                .unwrap_or("md")
                                .to_string(),
                        },
                        |highlight, _theme| highlight.to_format(),
                    )
                    .on_action(Message::Edit)
                    .height(Length::Fill),
            )
            .padding([5, spacing.space_xxs])
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
            PaneContent::Preview => container(
                scrollable(
                    markdown::view_with(
                        items,
                        markdown::Settings::default(),
                        &MarkdownViewer {
                            images: markdown_images,
                        },
                    )
                    .map(|m| match m {
                        markdown::MarkdownMessage::LinkClicked(url) => {
                            Message::LaunchUrl(url.to_string())
                        }
                        markdown::MarkdownMessage::ImageShown(url) => Message::ImageShown(url),
                    }),
                )
                .spacing(spacing.space_xxs),
            )
            .padding(spacing.space_xxs)
            .width(Length::Fill)
            .height(Length::Fill)
            .into(),
        };

        pane_grid::Content::new(pane_content)
            .title_bar(pane_grid::TitleBar::new(title_content))
            .class(theme::Container::Card)
    })
    .on_drag(Message::PaneDragged)
    .on_resize(10, Message::PaneResized)
    .spacing(spacing.space_xxs);

    let status_bar = {
        let file_path = match path.as_deref().and_then(Path::to_str) {
            Some(path) => text(path).size(12),
            None => text(fl!("new-file")).size(12),
        };

        let position = {
            let (line, column) = editor_content.cursor_position();
            text(format!("{}:{}", line + 1, column + 1)).size(12)
        };

        container(
            row![file_path, Space::with_width(Length::Fill), position]
                .padding(spacing.space_xxs)
                .spacing(spacing.space_xxs),
        )
        .width(Length::Fill)
        .class(theme::Container::Card)
    };

    container(column![pane_grid, status_bar].spacing(spacing.space_xxxs))
        .padding(spacing.space_xxs)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
