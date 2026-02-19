// SPDX-License-Identifier: GPL-3.0

use crate::app::app_menu::MenuAction;
use crate::app::context_page::ContextPage;
use crate::app::core::utils::project::ProjectNode;
use crate::app::core::utils::{self, CedillaToast};
use crate::app::dialogs::{DialogPage, DialogState};
use crate::app::widgets::{TextEditor, markdown, sensor, text_editor};
use crate::config::{AppTheme, CedillaConfig, ConfigInput, ShowState};
use crate::key_binds::key_binds;
use crate::{fl, icons};
use cosmic::app::context_drawer;
use cosmic::iced::{Alignment, Event, Length, Padding, Subscription, highlighter};
use cosmic::iced_core::keyboard::{Key, Modifiers};
use cosmic::iced_widget::{center, column, row, tooltip};
use cosmic::widget::menu::Action;
use cosmic::widget::{self, about::About, menu};
use cosmic::widget::{
    Space, ToastId, Toasts, button, container, nav_bar, pane_grid, responsive, scrollable,
    segmented_button, text, toaster,
};
use cosmic::{prelude::*, surface, theme};
use slotmap::Key as SlotMapKey;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub mod app_menu;
mod context_page;
mod core;
mod dialogs;
mod widgets;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application toasts
    toasts: Toasts<Message>,
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Application navbar
    nav_model: segmented_button::SingleSelectModel,
    /// Needed for navbar context menu func
    nav_bar_context_id: segmented_button::Entity,
    /// Dialog Pages of the Application
    dialog_pages: VecDeque<DialogPage>,
    /// Holds the state of the application dialogs
    dialog_state: DialogState,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// The about page for this app.
    about: About,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    /// Application Keyboard Modifiers
    modifiers: Modifiers,
    /// Application configuration handler
    config_handler: Option<cosmic::cosmic_config::Config>,
    /// Configuration data that persists between application runs.
    config: CedillaConfig,
    // Application Themes
    app_themes: Vec<String>,
    /// Currently selected path on the navbar (i need these for accurate file creadtion deletion...)
    selected_nav_path: Option<PathBuf>,
    /// Application State
    state: State,
}

/// Represents the Application State
#[allow(clippy::large_enum_variant)]
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
        /// Controls if the preview is hidden or not
        preview_state: PreviewState,
        /// snapshots of text (needed for ctrl-z)
        history: Vec<String>,
        /// current position in history
        history_index: usize,
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

/// State of the markdown preview in the app
enum PreviewState {
    Hidden,
    Shown,
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
    /// Callback after clicking something in the app menu
    MenuAction(app_menu::MenuAction),
    /// Needed for responsive menu bar
    Surface(surface::Action),
    /// Executes the appropiate cosmic binding on keyboard shortcut
    Key(Modifiers, Key),
    /// Updates the current state of keyboard modifiers
    Modifiers(Modifiers),
    /// Asks to execute various actions related to the application dialogs
    DialogAction(dialogs::DialogAction),

    /// Right click on a NavBar Item
    NavBarContext(segmented_button::Entity),
    /// Fired when a menu item is chosen
    NavMenuAction(NavMenuAction),

    /// Creates a new empty file (no path)
    NewFile,
    /// Creates a new markdown file in the vault
    NewVaultFile(String),
    /// Creates a new folder in the vault
    NewVaultFolder(String),
    /// Save the current file
    SaveFile,
    /// Callback after opening a new file
    OpenFile(Result<(PathBuf, Arc<String>), anywho::Error>),
    /// Callback after some action is performed on the text editor
    Edit(text_editor::Action),
    /// Callback after saving the current file
    FileSaved(Result<PathBuf, anywho::Error>),
    /// Deletes the given node entity of the navbar folder or file
    DeleteNode(cosmic::widget::segmented_button::Entity),
    /// Renames the given node entity of the navbar folder or file
    RenameNode(cosmic::widget::segmented_button::Entity, String),
    /// Move one node (to another one)
    MoveNode(cosmic::widget::segmented_button::Entity, PathBuf),

    /// Pane grid resized callback
    PaneResized(pane_grid::ResizeEvent),
    /// Pane grid dragged callback
    PaneDragged(pane_grid::DragEvent),
    /// Triggered when an image becomes visible
    ImageShown(markdown::Url),
    /// Callback after downloading/loading an image
    ImageLoaded(markdown::Url, Result<widget::image::Handle, String>),
    /// Apply formatting to selected text
    ApplyFormatting(utils::SelectionAction),
    /// Undo requested
    Undo,
    /// Redo requested
    Redo,

    /// Callback after input on the Config [`ContextPage`]
    ConfigInput(ConfigInput),
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
            Some(ImageState::Ready(handle)) => widget::image(handle.clone())
                .content_fit(cosmic::iced::ContentFit::Contain)
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into(),
            Some(ImageState::Loading) => text("Loading image...").into(),
            Some(ImageState::Failed) => text("Failed to load image").into(),
            None => sensor(text("Loading..."))
                .key_ref(url.as_str())
                .delay(Duration::from_millis(500))
                .on_show(|_size| markdown::MarkdownMessage::ImageShown(url.clone()))
                .into(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub enum NavMenuAction {
    DeleteNode(segmented_button::Entity),
    RenameNode(segmented_button::Entity),
    MoveNode(segmented_button::Entity),
}

impl cosmic::widget::menu::Action for NavMenuAction {
    type Message = cosmic::Action<Message>;
    fn message(&self) -> Self::Message {
        cosmic::Action::App(Message::NavMenuAction(*self))
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
            nav_model: nav_bar::Model::builder().build(),
            nav_bar_context_id: segmented_button::Entity::null(),
            context_page: ContextPage::default(),
            dialog_pages: VecDeque::default(),
            dialog_state: DialogState::default(),
            about,
            key_binds: key_binds(),
            modifiers: Modifiers::empty(),
            config_handler: flags.config_handler,
            config: flags.config,
            app_themes: vec![fl!("match-desktop"), fl!("dark"), fl!("light")],
            selected_nav_path: None,
            state: State::Loading,
        };

        // load vault
        let vault_path = PathBuf::from(&app.config.vault_path);
        app.open_vault_folder(&vault_path);

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

    /// Elements to pack at the end of the header bar.
    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let State::Ready { preview_state, .. } = &self.state else {
            return vec![];
        };

        let preview_button = match preview_state {
            PreviewState::Hidden => tooltip(
                widget::button::icon(icons::get_handle("show-symbolic", 18))
                    .on_press(Message::MenuAction(MenuAction::TogglePreview))
                    .class(theme::Button::Icon),
                container(text("Ctrl+H"))
                    .class(cosmic::theme::Container::Background)
                    .padding(8.),
                tooltip::Position::Bottom,
            ),
            PreviewState::Shown => tooltip(
                widget::button::icon(icons::get_handle("hide-symbolic", 18))
                    .on_press(Message::MenuAction(MenuAction::TogglePreview))
                    .class(theme::Button::Icon),
                container(text("Ctrl+H"))
                    .class(cosmic::theme::Container::Background)
                    .padding(8.),
                tooltip::Position::Bottom,
            ),
        };

        vec![container(preview_button).into()]
    }

    fn nav_bar(&self) -> Option<Element<'_, cosmic::action::Action<Message>>> {
        if !self.core().nav_bar_active() {
            return None;
        }

        let nav_model = self.nav_model()?;

        // if no valid items exist, render nothing
        nav_model.iter().next()?;

        let cosmic::cosmic_theme::Spacing {
            space_none,
            space_s,
            space_xxxs,
            ..
        } = self.core().system_theme().cosmic().spacing;

        let mut nav = segmented_button::vertical(nav_model)
            .button_height(space_xxxs + 20 /* line height */ + space_xxxs)
            .button_padding([space_s, space_xxxs, space_s, space_xxxs])
            .button_spacing(space_xxxs)
            .on_activate(|entity| cosmic::action::cosmic(cosmic::app::Action::NavBar(entity)))
            .on_context(|entity| cosmic::Action::App(Message::NavBarContext(entity)))
            .context_menu(self.nav_context_menu(self.nav_bar_context_id))
            .spacing(space_none)
            .style(theme::SegmentedButton::FileNav)
            .apply(widget::container)
            .padding(space_s)
            .width(Length::Shrink);

        if !self.core().is_condensed() {
            nav = nav.max_width(280);
        }

        Some(
            nav.apply(widget::scrollable)
                .apply(widget::container)
                .height(Length::Fill)
                .class(theme::Container::custom(nav_bar::nav_bar_style))
                .into(),
        )
    }

    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav_model)
    }

    fn nav_context_menu(
        &self,
        entity: widget::nav_bar::Id,
    ) -> Option<Vec<widget::menu::Tree<cosmic::Action<Message>>>> {
        if entity.is_null() {
            return Some(vec![]);
        }

        // no context menu for root node
        if self.nav_model.indent(entity).unwrap_or(0) == 0 {
            return Some(vec![]);
        }

        let node = self.nav_model.data::<ProjectNode>(entity)?;

        let mut items = Vec::with_capacity(1);

        match node {
            ProjectNode::File { .. } => {
                items.push(cosmic::widget::menu::Item::Button(
                    fl!("delete"),
                    None,
                    NavMenuAction::DeleteNode(entity),
                ));
                items.push(cosmic::widget::menu::Item::Button(
                    fl!("rename"),
                    None,
                    NavMenuAction::RenameNode(entity),
                ));
                items.push(cosmic::widget::menu::Item::Button(
                    fl!("move-to"),
                    None,
                    NavMenuAction::MoveNode(entity),
                ));
            }
            ProjectNode::Folder { .. } => {
                items.push(cosmic::widget::menu::Item::Button(
                    fl!("delete"),
                    None,
                    NavMenuAction::DeleteNode(entity),
                ));
                items.push(cosmic::widget::menu::Item::Button(
                    fl!("rename"),
                    None,
                    NavMenuAction::RenameNode(entity),
                ));
                items.push(cosmic::widget::menu::Item::Button(
                    fl!("move-to"),
                    None,
                    NavMenuAction::MoveNode(entity),
                ));
            }
        }

        Some(cosmic::widget::menu::items(
            &std::collections::HashMap::new(),
            items,
        ))
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Message>> {
        let node_opt = match self.nav_model.data_mut::<ProjectNode>(id) {
            Some(node) => {
                if let ProjectNode::Folder { open, .. } = node {
                    *open = !*open;
                }
                Some(node.clone())
            }
            None => None,
        };

        match node_opt {
            Some(node) => {
                self.nav_model.icon_set(id, node.icon(18));

                match node {
                    ProjectNode::Folder { path, open, .. } => {
                        // store selected directory
                        self.selected_nav_path = Some(path.clone());

                        let position = self.nav_model.position(id).unwrap_or(0);
                        let indent = self.nav_model.indent(id).unwrap_or(0);

                        if open {
                            self.open_folder(&path, position + 1, indent + 1);
                        } else {
                            while let Some(child_id) = self.nav_model.entity_at(position + 1) {
                                if self.nav_model.indent(child_id).unwrap_or(0) > indent {
                                    self.nav_model.remove(child_id);
                                } else {
                                    break;
                                }
                            }
                        }
                        Task::none()
                    }
                    ProjectNode::File { path, .. } => {
                        // store parent directory of selected file
                        self.selected_nav_path = path.parent().map(|p| p.to_path_buf());

                        Task::perform(utils::files::load_file(path), |res| {
                            cosmic::action::app(Message::OpenFile(res))
                        })
                    }
                }
            }
            None => Task::none(),
        }
    }

    /// Display a dialog if requested.
    fn dialog(&self) -> Option<Element<'_, Message>> {
        let dialog_page = self.dialog_pages.front()?;
        dialog_page.display(&self.dialog_state)
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
                preview_state,
                ..
            } => cedilla_main_view(
                &self.config,
                path,
                editor_content,
                markdown_images,
                items,
                is_dirty,
                panes,
                preview_state,
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
            // Watch for key_bind inputs
            cosmic::iced::event::listen_with(|event, status, _| match event {
                Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) => match status {
                    cosmic::iced::event::Status::Ignored => Some(Message::Key(modifiers, key)),
                    cosmic::iced::event::Status::Captured => None,
                },
                Event::Keyboard(cosmic::iced::keyboard::Event::ModifiersChanged(modifiers)) => {
                    Some(Message::Modifiers(modifiers))
                }
                _ => None,
            }),
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
            Message::LaunchUrl(url) => match open::that_detached(&url) {
                Ok(()) => Task::none(),
                Err(err) => {
                    eprintln!("failed to open {url:?}: {err}");
                    Task::none()
                }
            },
            Message::MenuAction(action) => {
                let State::Ready { preview_state, .. } = &mut self.state else {
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
                    MenuAction::NewVaultFile => self.update(Message::DialogAction(
                        dialogs::DialogAction::OpenNewVaultFileDialog,
                    )),
                    MenuAction::NewVaultFolder => self.update(Message::DialogAction(
                        dialogs::DialogAction::OpenNewVaultFolderDialog,
                    )),
                    MenuAction::SaveFile => self.update(Message::SaveFile),
                    MenuAction::TogglePreview => {
                        match preview_state {
                            PreviewState::Hidden => *preview_state = PreviewState::Shown,
                            PreviewState::Shown => *preview_state = PreviewState::Hidden,
                        }
                        Task::none()
                    }
                    MenuAction::Undo => self.update(Message::Undo),
                    MenuAction::Redo => self.update(Message::Redo),
                }
            }
            Message::Surface(a) => {
                cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::Surface(a)))
            }
            Message::Key(modifiers, key) => {
                for (key_bind, action) in self.key_binds.iter() {
                    if key_bind.matches(modifiers, &key) {
                        return self.update(action.message());
                    }
                }
                Task::none()
            }
            Message::Modifiers(modifiers) => {
                self.modifiers = modifiers;
                Task::none()
            }
            Message::DialogAction(action) => {
                let State::Ready { .. } = &mut self.state else {
                    return Task::none();
                };

                action.execute(&mut self.dialog_pages, &self.dialog_state)
            }

            Message::NavBarContext(entity) => {
                self.nav_bar_context_id = entity;
                Task::none()
            }
            Message::NavMenuAction(action) => {
                self.nav_bar_context_id = segmented_button::Entity::null();
                match action {
                    NavMenuAction::DeleteNode(entity) => self.update(Message::DialogAction(
                        dialogs::DialogAction::OpenDeleteNodeDialog(entity),
                    )),
                    NavMenuAction::RenameNode(entity) => self.update(Message::DialogAction(
                        dialogs::DialogAction::OpenRenameNodeDialog(entity),
                    )),
                    NavMenuAction::MoveNode(entity) => {
                        let vault_path = PathBuf::from(&self.config.vault_path);
                        self.dialog_state.available_folders =
                            self.collect_all_folders(&vault_path, entity);

                        self.update(Message::DialogAction(
                            dialogs::DialogAction::OpenMoveNodeDialog(entity),
                        ))
                    }
                }
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
                    preview_state: PreviewState::Shown,
                    history: Vec::new(),
                    history_index: 0,
                };
                Task::none()
            }
            Message::NewVaultFile(file_name) => {
                let dir = self.selected_directory();

                // find a name that doesn't already exist
                let file_path = {
                    let base = dir.join(format!("{}.md", file_name));
                    if !base.exists() {
                        base
                    } else {
                        let mut i = 1;
                        loop {
                            let candidate = dir.join(format!("{}-{}.md", file_name, i));
                            if !candidate.exists() {
                                break candidate;
                            }
                            i += 1;
                        }
                    }
                };

                // create the file on disk
                if let Err(e) = std::fs::write(&file_path, "") {
                    return self.update(Message::AddToast(CedillaToast::new(e)));
                }

                // insert file to navbar
                self.insert_file_node(&file_path, &dir);

                // Create initial pane configuration with editor on left, preview on right
                let (mut panes, first_pane) = pane_grid::State::new(PaneContent::Editor);
                panes.split(pane_grid::Axis::Vertical, first_pane, PaneContent::Preview);

                self.state = State::Ready {
                    path: Some(file_path),
                    editor_content: text_editor::Content::new(),
                    markdown_images: HashMap::new(),
                    items: vec![],
                    is_dirty: true,
                    panes,
                    preview_state: PreviewState::Shown,
                    history: Vec::new(),
                    history_index: 0,
                };

                Task::none()
            }
            Message::NewVaultFolder(folder_name) => {
                let dir = self.selected_directory();

                let folder_path = {
                    let base = dir.join(&folder_name);
                    if !base.exists() {
                        base
                    } else {
                        let mut i = 1;
                        loop {
                            let candidate = dir.join(format!("{}-{}", folder_name, i));
                            if !candidate.exists() {
                                break candidate;
                            }
                            i += 1;
                        }
                    }
                };

                // create the file on disk
                if let Err(e) = std::fs::create_dir(&folder_path) {
                    return self.update(Message::AddToast(CedillaToast::new(e)));
                }

                // insert folder to navbar
                self.insert_folder_node(&folder_path, &dir);

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
                let vault_path = self.config.vault_path.clone();

                Task::perform(
                    async move {
                        match path {
                            // We're editing an alreaday existing file
                            Some(path) => Some(utils::files::save_file(path, content).await),
                            // We want to save a new file
                            None => {
                                match utils::files::open_markdown_file_saver(vault_path).await {
                                    Some(path) => {
                                        Some(utils::files::save_file(path.into(), content).await)
                                    }
                                    // Error selecting where to save the file
                                    None => None,
                                }
                            }
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
                        preview_state: PreviewState::Shown,
                        history: vec![content.to_string()],
                        history_index: 0,
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
                    history,
                    history_index,
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                let was_edit = action.is_edit();
                editor_content.perform(action);
                *items = markdown::parse(editor_content.text().as_ref()).collect();

                if was_edit {
                    *is_dirty = true;
                    let current_text = editor_content.text();

                    history.truncate(*history_index + 1);
                    history.push(current_text);
                    *history_index = history.len() - 1;

                    // keep only the last 100 snapshots
                    if history.len() > 100 {
                        history.remove(0);
                    } else {
                        *history_index = history.len() - 1;
                    }
                }

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
            Message::DeleteNode(entity) => {
                let Some(node) = self.nav_model.data::<ProjectNode>(entity).cloned() else {
                    return Task::none();
                };

                let path = match &node {
                    ProjectNode::File { path, .. } => path.clone(),
                    ProjectNode::Folder { path, .. } => path.clone(),
                };

                let delete_result = match &node {
                    ProjectNode::File { .. } => std::fs::remove_file(&path),
                    ProjectNode::Folder { .. } => std::fs::remove_dir_all(&path),
                };

                if let Err(e) = delete_result {
                    return self.update(Message::AddToast(CedillaToast::new(e)));
                }

                // remove from nav model
                self.remove_nav_node(&path);

                // if the deleted file was currently open, create a new empty file
                if let State::Ready {
                    path: open_path, ..
                } = &self.state
                    && open_path.as_deref() == Some(&path)
                {
                    return self.update(Message::NewFile);
                }

                Task::none()
            }
            Message::RenameNode(entity, new_name) => {
                let Some(node) = self.nav_model.data::<ProjectNode>(entity).cloned() else {
                    return Task::none();
                };

                let old_path = match &node {
                    ProjectNode::File { path, .. } | ProjectNode::Folder { path, .. } => {
                        path.clone()
                    }
                };

                let new_name = match &node {
                    ProjectNode::File { .. } => {
                        if new_name.ends_with(".md") {
                            new_name
                        } else {
                            format!("{}.md", new_name)
                        }
                    }
                    ProjectNode::Folder { .. } => new_name,
                };

                let new_path = match old_path.parent() {
                    Some(parent) => parent.join(&new_name),
                    None => return Task::none(),
                };

                if new_path == old_path {
                    return Task::none();
                }

                if new_path.exists() {
                    return self.update(Message::AddToast(CedillaToast::new(format!(
                        "A file or folder named {:?} already exists",
                        new_name
                    ))));
                }

                if let Err(e) = std::fs::rename(&old_path, &new_path) {
                    return self.update(Message::AddToast(CedillaToast::new(e)));
                }

                self.rename_nav_node(&old_path, &new_path, &new_name);

                // update the open editor state if the open file was inside the renamed path
                #[allow(clippy::collapsible_if)]
                if let State::Ready {
                    path: open_path, ..
                } = &mut self.state
                {
                    if let Some(current) = open_path.as_deref() {
                        if current.starts_with(&old_path) {
                            let suffix = current.strip_prefix(&old_path).unwrap().to_path_buf();
                            *open_path = Some(if suffix == std::path::Path::new("") {
                                new_path.clone()
                            } else {
                                new_path.join(suffix)
                            });
                        }
                    }
                }
                Task::none()
            }
            Message::MoveNode(source_entity, target_path) => {
                let source_path = match self.nav_model.data::<ProjectNode>(source_entity) {
                    Some(ProjectNode::File { path, .. } | ProjectNode::Folder { path, .. }) => {
                        path.clone()
                    }
                    None => return Task::none(),
                };

                let file_name = match source_path.file_name() {
                    Some(n) => n,
                    None => return Task::none(),
                };
                let dest = target_path.join(file_name);

                if source_path == target_path || dest == source_path {
                    return Task::none();
                }

                if let Err(e) = std::fs::rename(&source_path, &dest) {
                    return self.update(Message::AddToast(CedillaToast::new(e)));
                }

                let target_entity = self.nav_model.iter().find(|&id| {
                    matches!(
                        self.nav_model.data::<ProjectNode>(id),
                        Some(ProjectNode::Folder { path, .. }) if *path == target_path
                    )
                });

                if let Some(target_entity) = target_entity {
                    self.move_nav_node(source_entity, target_entity, &dest);
                } else {
                    // target folder was never opened, reload from disk
                    let vault_path = PathBuf::from(&self.config.vault_path);
                    self.nav_model.clear();
                    self.open_vault_folder(&vault_path);
                }

                if let State::Ready {
                    path: open_path, ..
                } = &mut self.state
                    && open_path.as_deref() == Some(&source_path)
                {
                    *open_path = Some(dest);
                }

                Task::none()
            }

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
            Message::ApplyFormatting(action) => self.apply_formatting_to_selection(action),
            Message::Undo => {
                let State::Ready {
                    editor_content,
                    is_dirty,
                    items,
                    history,
                    history_index,
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                if *history_index > 0 {
                    *history_index -= 1;
                    let snapshot = history[*history_index].clone();
                    *editor_content = text_editor::Content::with_text(&snapshot);
                    *items = markdown::parse(&snapshot).collect();
                    *is_dirty = *history_index != 0;
                }
                Task::none()
            }
            Message::Redo => {
                let State::Ready {
                    editor_content,
                    items,
                    history,
                    history_index,
                    ..
                } = &mut self.state
                else {
                    return Task::none();
                };

                if *history_index + 1 < history.len() {
                    *history_index += 1;
                    let snapshot = history[*history_index].clone();
                    *editor_content = text_editor::Content::with_text(&snapshot);
                    *items = markdown::parse(&snapshot).collect();
                }
                Task::none()
            }

            #[allow(clippy::collapsible_if)]
            Message::ConfigInput(input) => match input {
                ConfigInput::UpdateTheme(index) => {
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
                ConfigInput::HelperHeaderBarShowState(show_state) => {
                    if let Some(handler) = &self.config_handler {
                        if let Err(err) =
                            self.config.set_show_helper_header_bar(handler, show_state)
                        {
                            eprintln!("{err}");
                            // even if it fails we update the config (it won't get saved after restart)
                            let mut old_config = self.config.clone();
                            old_config.show_helper_header_bar = show_state;
                            self.config = old_config;
                        }
                    }
                    Task::none()
                }
                ConfigInput::StatusBarShowState(show_state) => {
                    if let Some(handler) = &self.config_handler {
                        if let Err(err) = self.config.set_show_status_bar(handler, show_state) {
                            eprintln!("{err}");
                            // even if it fails we update the config (it won't get saved after restart)
                            let mut old_config = self.config.clone();
                            old_config.show_status_bar = show_state;
                            self.config = old_config;
                        }
                    }
                    Task::none()
                }
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
                        |t| Message::ConfigInput(ConfigInput::UpdateTheme(t)),
                    )),
                )
                .into(),
            widget::settings::section()
                .title(fl!("view"))
                .add(
                    widget::settings::item::builder(fl!("help-bar")).control(widget::dropdown(
                        ShowState::all_labels(),
                        Some(self.config.show_helper_header_bar.to_index()),
                        |index| {
                            Message::ConfigInput(ConfigInput::HelperHeaderBarShowState(
                                ShowState::from_index(index),
                            ))
                        },
                    )),
                )
                .add(
                    widget::settings::item::builder(fl!("status-bar")).control(widget::dropdown(
                        ShowState::all_labels(),
                        Some(self.config.show_status_bar.to_index()),
                        |index| {
                            Message::ConfigInput(ConfigInput::StatusBarShowState(
                                ShowState::from_index(index),
                            ))
                        },
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
#[allow(clippy::too_many_arguments)]
fn cedilla_main_view<'a>(
    app_config: &'a CedillaConfig,
    path: &'a Option<PathBuf>,
    editor_content: &'a text_editor::Content,
    markdown_images: &'a HashMap<url::Url, ImageState>,
    items: &'a [markdown::Item],
    is_dirty: &'a bool,
    panes: &'a pane_grid::State<PaneContent>,
    preview_state: &'a PreviewState,
) -> Element<'a, Message> {
    let spacing = theme::active().cosmic().spacing;

    let create_editor = || {
        container(responsive(|size| {
            scrollable(
                TextEditor::new(editor_content)
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
                    .padding(0)
                    .on_action(Message::Edit),
            )
            .height(Length::Fixed(size.height - 5.)) // This is a bit of a workaround but it works
            .into()
        }))
        .padding([5, spacing.space_xxs])
        .width(Length::Fill)
        .height(Length::Fill)
    };

    let create_title = |icon_name: &str, title: String| {
        row![
            widget::icon::from_name(icon_name).size(16),
            text(title).size(14),
        ]
        .spacing(spacing.space_xxs)
        .padding([5, spacing.space_xxs])
        .align_y(Alignment::Center)
    };

    let main_content: Element<'a, Message> = if matches!(preview_state, PreviewState::Hidden) {
        // Editor only view
        container(column![
            container(create_title("text-editor-symbolic", fl!("editor")))
                .padding(3.)
                .width(Length::Fill)
                .class(theme::Container::Card),
            create_editor()
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .class(theme::Container::Card)
        .into()
    } else {
        // Pane grid with editor and preview
        pane_grid::PaneGrid::new(panes, |_pane, content, _is_focused| {
            let (title, icon_name) = match content {
                PaneContent::Editor => (fl!("editor"), "text-editor-symbolic"),
                PaneContent::Preview => (fl!("preview"), "view-paged-symbolic"),
            };

            let pane_content: Element<'a, Message> = match content {
                PaneContent::Editor => create_editor().into(),
                PaneContent::Preview => container(scrollable(
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
                ))
                .padding(spacing.space_xxs)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            };

            pane_grid::Content::new(pane_content)
                .title_bar(pane_grid::TitleBar::new(create_title(icon_name, title)).padding(3.))
                .class(theme::Container::Card)
        })
        .on_drag(Message::PaneDragged)
        .on_resize(10, Message::PaneResized)
        .spacing(spacing.space_xxs)
        .into()
    };

    let status_bar: Element<Message> = {
        let file_path = match path.as_deref().and_then(Path::to_str) {
            Some(path) => text(path).size(12),
            None => text(fl!("new-file")).size(12),
        };

        let dirty_indicator = if *is_dirty {
            text("•").size(12)
        } else {
            text("").size(12)
        };

        let position = {
            let (line, column) = editor_content.cursor_position();
            text(format!("{}:{}", line + 1, column + 1)).size(12)
        };

        container(
            row![
                file_path,
                dirty_indicator,
                Space::with_width(Length::Fill),
                position
            ]
            .padding(spacing.space_xxs)
            .spacing(spacing.space_xxs),
        )
        .width(Length::Fill)
        .class(theme::Container::Card)
        .into()
    };

    let helper_header_bar = {
        container(
            row![
                button::icon(icons::get_handle("helperbar/bold-symbolic", 18))
                    .on_press(Message::ApplyFormatting(utils::SelectionAction::Bold)),
                button::icon(icons::get_handle("helperbar/italic-symbolic", 18))
                    .on_press(Message::ApplyFormatting(utils::SelectionAction::Italic)),
                Space::new(18., 0.),
                button::icon(icons::get_handle("helperbar/link-symbolic", 18))
                    .on_press(Message::ApplyFormatting(utils::SelectionAction::Hyperlink)),
                button::icon(icons::get_handle("helperbar/code-symbolic", 18))
                    .on_press(Message::ApplyFormatting(utils::SelectionAction::Code)),
                button::icon(icons::get_handle("helperbar/image-symbolic", 18))
                    .on_press(Message::ApplyFormatting(utils::SelectionAction::Image)),
                Space::new(18., 0.),
                button::icon(icons::get_handle("helperbar/bulleted-list-symbolic", 18)).on_press(
                    Message::ApplyFormatting(utils::SelectionAction::BulletedList)
                ),
                button::icon(icons::get_handle("helperbar/numbered-list-symbolic", 18)).on_press(
                    Message::ApplyFormatting(utils::SelectionAction::NumberedList)
                ),
                button::icon(icons::get_handle("helperbar/checked-list-symbolic", 18)).on_press(
                    Message::ApplyFormatting(utils::SelectionAction::CheckboxList)
                ),
                button::icon(icons::get_handle("helperbar/heading-symbolic", 18))
                    .on_press(Message::ApplyFormatting(utils::SelectionAction::Heading1)),
                button::icon(icons::get_handle("helperbar/rule-symbolic", 18))
                    .on_press(Message::ApplyFormatting(utils::SelectionAction::Rule)),
            ]
            .padding(spacing.space_xxs)
            .spacing(spacing.space_xxs),
        )
        .width(Length::Fill)
        .class(theme::Container::Card)
    };

    let content_column = match app_config.show_helper_header_bar {
        ShowState::Show => column![helper_header_bar, main_content],
        ShowState::Hide => column![main_content],
    }
    .spacing(spacing.space_xxxs);

    let content_column = if let ShowState::Show = &app_config.show_status_bar {
        content_column.extend([status_bar])
    } else {
        content_column
    };

    container(content_column)
        .padding(
            Padding::new(spacing.space_xxs as f32)
                .left(0.)
                .right(0.)
                .top(0.),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
