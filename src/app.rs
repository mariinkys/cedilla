// SPDX-License-Identifier: GPL-3.0

use crate::app::app_menu::MenuAction;
use crate::app::context_page::ContextPage;
use crate::app::core::utils::{self, CedillaToast};
use crate::config::{AppTheme, CedillaConfig};
use crate::fl;
use cosmic::app::context_drawer;
use cosmic::iced::{Length, Subscription, highlighter};
use cosmic::iced_widget::{center, column, row};
use cosmic::widget::{self, about::About, menu};
use cosmic::widget::{
    Space, ToastId, Toasts, container, markdown, scrollable, text, text_editor, toaster,
};
use cosmic::{prelude::*, surface};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod app_menu;
mod context_page;
mod core;

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
        /// Markdown preview items
        items: Vec<markdown::Item>,
        /// Track if any changes have been made to the current file
        is_dirty: bool,
    },
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
                items,
                is_dirty,
            } => cedilla_main_view(path, editor_content, items, is_dirty),
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
                self.state = State::Ready {
                    path: None,
                    editor_content: text_editor::Content::new(),
                    items: vec![],
                    is_dirty: true,
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
                    self.state = State::Ready {
                        path: Some(path),
                        editor_content: text_editor::Content::with_text(content.as_ref()),
                        items: markdown::parse(content.as_ref()).collect(),
                        is_dirty: false,
                    };
                    Task::none()
                }
                Err(e) => self.update(Message::AddToast(CedillaToast::new(e))),
            },
            Message::Edit(action) => {
                let State::Ready {
                    editor_content,
                    is_dirty,
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                *is_dirty = *is_dirty || action.is_edit();
                editor_content.perform(action);

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
    items: &'a [markdown::Item],
    _is_dirty: &'a bool,
) -> Element<'a, Message> {
    let editor = text_editor(editor_content)
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
        .height(Length::Fill);

    let preview = markdown(
        items,
        markdown::Settings::default(),
        markdown::Style::from_palette(cosmic::iced::Theme::palette(&cosmic::iced::Theme::Dark)),
    )
    .map(|u| Message::LaunchUrl(u.to_string()));

    let status_bar = {
        let file_path = match path.as_deref().and_then(Path::to_str) {
            Some(path) => text(path),
            None => text(fl!("new-file")),
        };

        let position = {
            let (line, column) = editor_content.cursor_position();
            text(format!("{}:{}", line + 1, column + 1))
        };

        row![file_path, Space::with_width(Length::Fill), position]
    };

    container(
        column![
            row![
                editor,
                scrollable(preview).spacing(10.).height(Length::Fill)
            ]
            .spacing(10.),
            status_bar
        ]
        .spacing(10.),
    )
    .padding(5.)
    .into()
}
