// SPDX-License-Identifier: GPL-3.0

use crate::app::app_menu::MenuAction;
use crate::app::context_page::ContextPage;
use crate::app::core::editor::EditorState;
use crate::app::core::preview::MarkdownPreview;
use crate::app::core::project::ProjectNode;
use crate::app::core::utils::{self, CedillaToast, Image};
use crate::app::dialogs::{DialogPage, DialogState};
use crate::config::{AppTheme, BoolState, CedillaConfig, ConfigInput, ShowState};
use crate::key_binds::key_binds;
use crate::{fl, icons};
use cosmic::app::context_drawer;
use cosmic::iced::{Alignment, Event, Length, Padding, Subscription, highlighter, window};
use cosmic::iced_core::keyboard::{Key, Modifiers};
use cosmic::iced_widget::{center, column, row, scrollable, tooltip};
use cosmic::widget::space::horizontal;
use cosmic::widget::{self, about::About, menu};
use cosmic::widget::{
    ToastId, Toasts, button, container, nav_bar, pane_grid, responsive, segmented_button, text,
    text_input, toaster,
};
use cosmic::{prelude::*, surface, theme};
use frostmark::{MarkWidget, UpdateMsg};
use slotmap::Key as SlotMapKey;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use widgets::{TextEditor, text_editor};

pub mod app_menu;
mod context_page;
mod core;
mod dialogs;
mod update;

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
    /// Gotenberg client, needed for Pdf exporting
    gotenberg_client: gotenberg_pdf::Client,
    /// Application State
    state: State,
}

/// Represents the Application State
#[allow(clippy::large_enum_variant)]
enum State {
    Loading,
    Ready {
        /// Holds the state for editor related fields
        editor: EditorState,
        /// Holds the state for preview related fields
        preview: MarkdownPreview,
        /// Pane grid state
        panes: pane_grid::State<PaneContent>,
        /// Controls if the preview is hidden or not
        preview_state: PreviewState,
    },
}

/// Content type for each pane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PaneContent {
    Editor,
    Preview,
}

/// State of the markdown preview in the app
#[derive(Debug, Clone)]
pub enum PreviewState {
    Hidden,
    Shown,
}

/// Possible actions after asking to discard changes in the warning [`DialogPage`]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscardChangesAction {
    CloseApp,
    OpenFile(PathBuf),
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
    /// Needed for responsive menu bar
    Surface(surface::Action),
    /// Executes the appropiate cosmic binding on keyboard shortcut
    Key(Modifiers, Key),
    /// Updates the current state of keyboard modifiers
    Modifiers(Modifiers),

    /// Callback after clicking something in the app menu
    MenuAction(app_menu::MenuAction),

    /// Asks to execute various actions related to the application dialogs
    DialogAction(dialogs::DialogAction),
    /// Right click on a NavBar Item
    NavBarContext(segmented_button::Entity),
    /// Fired when a menu item is chosen
    NavMenuAction(NavMenuAction),

    /// Startup Message, checks config and set's the state as needed
    Startup,
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
    /// Callback after saving the current file
    FileSaved(Result<PathBuf, anywho::Error>),
    /// Callback after asking to close a file discarding changes
    DiscardChanges(DiscardChangesAction),

    /// Deletes the given node entity of the navbar folder or file
    DeleteNode(cosmic::widget::segmented_button::Entity),
    /// Renames the given node entity of the navbar folder or file
    RenameNode(cosmic::widget::segmented_button::Entity, String),
    /// Move one node (to another one)
    MoveNode(cosmic::widget::segmented_button::Entity, PathBuf),
    /// Ask to move the main vault to another path
    MoveVault,
    /// Callback after asking to move the vault
    VaultMoved(Result<PathBuf, anywho::Error>),

    /// Callback after some action is performed on the text editor
    Edit(text_editor::Action),
    /// Apply formatting to selected text
    ApplyFormatting(utils::SelectionAction),
    /// Undo requested
    Undo,
    /// Redo requested
    Redo,

    /// Update the HTML renderer state
    UpdateMarkState(UpdateMsg),
    /// Callback after a Markdown image has been downloaded
    ImageDownloaded(Result<Image, anywho::Error>),
    /// Set's the preview to the desired state
    SetPreviewState(PreviewState),
    /// Pane grid resized callback
    PaneResized(pane_grid::ResizeEvent),
    /// Pane grid dragged callback
    PaneDragged(pane_grid::DragEvent),
    /// Scrollable position changed
    ScrollChanged(widget::Id, scrollable::Viewport),

    /// Callback after input on the Config [`ContextPage`]
    ConfigInput(ConfigInput),

    /// Export current document to PDF
    ExportPDF,
    /// Callback after using asks to close the app
    AppCloseRequested(window::Id),
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

        let gotenberg_url = flags.config.gotenberg_url.clone();

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
            gotenberg_client: gotenberg_pdf::Client::new(&gotenberg_url),
            state: State::Loading,
        };

        // restore last navbar app state
        match app.config.last_navbar_showstate {
            ShowState::Show => app.core.nav_bar_set_toggled(true),
            ShowState::Hide => app.core.nav_bar_set_toggled(false),
        }

        // load vault
        let vault_path = PathBuf::from(&app.config.vault_path);
        if vault_path.exists() && vault_path.is_dir() {
            app.open_vault_folder(&vault_path);
        } else {
            eprintln!("Vault not found, trying to fallback to default path");
            let default_vault_path = PathBuf::from(CedillaConfig::default().vault_path);

            #[allow(clippy::collapsible_if)]
            if let Some(handler) = &app.config_handler {
                if let Err(err) = app
                    .config
                    .set_vault_path(handler, default_vault_path.to_string_lossy().to_string())
                {
                    eprintln!("{err}");
                }
            }

            app.config.vault_path = default_vault_path.to_string_lossy().to_string();
            app.open_vault_folder(&default_vault_path);
        }

        // Startup tasks.
        let tasks = vec![
            app.update_title(),
            cosmic::command::set_theme(app.config.app_theme.theme()),
            Task::done(cosmic::action::app(Message::Startup)),
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
            nav = nav.width(Length::Fixed(280.)); // remove if it ever get's fixed (needed right now in iced-rebase branch)
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

        // no need to use this for now as we don't have different context menu options for files and folders
        //let node = self.nav_model.data::<ProjectNode>(entity)?;

        let mut items = Vec::with_capacity(1);

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

        Some(cosmic::widget::menu::items(
            &std::collections::HashMap::new(),
            items,
        ))
    }

    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &self.state else {
            return Task::none();
        };

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
                        // we don't do this here anymore because if the user discard changes... we don't want to change the selected_nav_path
                        // it's better to do this after opening a file if the path opened is inside the vault
                        //store parent directory of selected file
                        //self.selected_nav_path = path.parent().map(|p| p.to_path_buf());

                        if editor.is_dirty {
                            if editor.needs_confirmation() {
                                Task::done(cosmic::action::app(Message::DialogAction(
                                    dialogs::DialogAction::OpenConfirmCloseFileDialog(
                                        DiscardChangesAction::OpenFile(path),
                                    ),
                                )))
                            } else {
                                Task::perform(utils::files::load_file(path), |res| {
                                    cosmic::action::app(Message::OpenFile(res))
                                })
                            }
                        } else {
                            Task::perform(utils::files::load_file(path), |res| {
                                cosmic::action::app(Message::OpenFile(res))
                            })
                        }
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
                editor,
                preview,
                panes,
                preview_state,
                ..
            } => cedilla_main_view(&self.config, editor, preview, panes, preview_state),
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

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::AppCloseRequested(id))
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            // UI
            Message::CloseToast(id) => self.handle_close_toast(id),
            Message::AddToast(toast) => self.handle_add_toast(toast),
            Message::LaunchUrl(url) => self.handle_launch_url(url),
            Message::ToggleContextPage(page) => self.handle_toggle_context_page(page),
            Message::UpdateConfig(config) => self.handle_update_config(config),
            Message::Surface(a) => self.handle_surface(a),
            Message::Key(modifiers, key) => self.handle_key(modifiers, key),
            Message::Modifiers(modifiers) => self.handle_modifiers(modifiers),

            // Menu
            Message::MenuAction(action) => self.handle_menu_action(action),

            // Dialog & NavBar
            Message::DialogAction(action) => self.handle_dialog_action(action),
            Message::NavBarContext(entity) => self.handle_nav_bar_context(entity),
            Message::NavMenuAction(action) => self.handle_nav_menu_action(action),

            // File
            Message::Startup => self.handle_startup(),
            Message::NewFile => self.handle_new_file(),
            Message::NewVaultFile(name) => self.handle_new_vault_file(name),
            Message::NewVaultFolder(name) => self.handle_new_vault_folder(name),
            Message::SaveFile => self.handle_save_file(),
            Message::OpenFile(result) => self.handle_open_file(result),
            Message::FileSaved(result) => self.handle_file_saved(result),
            Message::DiscardChanges(action) => self.handle_discard_changes(action),

            // Vault / Node
            Message::DeleteNode(entity) => self.handle_delete_node(entity),
            Message::RenameNode(entity, name) => self.handle_rename_node(entity, name),
            Message::MoveNode(entity, path) => self.handle_move_node(entity, path),
            Message::MoveVault => self.handle_move_vault(),
            Message::VaultMoved(result) => self.handle_vault_moved(result),

            // Editor
            Message::Edit(action) => self.handle_edit(action),
            Message::ApplyFormatting(action) => self.handle_apply_formatting(action),
            Message::Undo => self.handle_undo(),
            Message::Redo => self.handle_redo(),

            // Preview / Pane
            Message::UpdateMarkState(msg) => self.handle_update_mark_state(msg),
            Message::ImageDownloaded(result) => self.handle_image_downloaded(result),
            Message::SetPreviewState(state) => self.handle_set_preview_state(state),
            Message::PaneResized(event) => self.handle_pane_resized(event),
            Message::PaneDragged(event) => self.handle_pane_dragged(event),
            Message::ScrollChanged(id, viewport) => self.handle_scroll_changed(id, viewport),

            // Config
            Message::ConfigInput(input) => self.handle_config_input(input),

            // Others
            Message::ExportPDF => self.handle_export_pdf(),
            Message::AppCloseRequested(id) => self.handle_app_close_requested(id),
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
                .title(fl!("general"))
                .add(
                    widget::settings::item::builder(fl!("move-vault"))
                        .description(fl!(
                            "current-location",
                            location = self.config.vault_path.to_string()
                        ))
                        .control(
                            widget::button::destructive(fl!("move-vault"))
                                .on_press(Message::MoveVault),
                        ),
                )
                .add(
                    widget::settings::item::builder(fl!("last-file")).control(widget::dropdown(
                        BoolState::all_labels(),
                        Some(self.config.open_last_file.to_index()),
                        |index| {
                            Message::ConfigInput(ConfigInput::OpenLastFile(BoolState::from_index(
                                index,
                            )))
                        },
                    )),
                )
                .add(
                    widget::settings::item::builder(fl!("scrollbar-sync")).control(
                        widget::dropdown(
                            BoolState::all_labels(),
                            Some(self.config.scrollbar_sync.to_index()),
                            |index| {
                                Message::ConfigInput(ConfigInput::ScrollbarSync(
                                    BoolState::from_index(index),
                                ))
                            },
                        ),
                    ),
                )
                .add(
                    widget::settings::item::builder(fl!("text-size"))
                        .description(format!("{}px", self.config.text_size))
                        .control(
                            widget::slider(6..=30, self.config.text_size as u16, |v| {
                                Message::ConfigInput(ConfigInput::UpdateTextSize(v))
                            })
                            .step(1u16),
                        ),
                )
                .add(
                    cosmic::widget::column::with_children(vec![
                        column![
                            text::body(fl!("pdf-exporting")),
                            text::caption(fl!("gotenberg-url"))
                        ]
                        .into(),
                        text_input(fl!("gotenberg-url"), &self.config.gotenberg_url)
                            .on_input(|v| Message::ConfigInput(ConfigInput::GotenbergUrlInput(v)))
                            .into(),
                        row![
                            button::text(fl!("more-info")).on_press(Message::LaunchUrl(
                                String::from("https://gotenberg.dev/")
                            )),
                            horizontal(),
                            button::suggested(fl!("apply"))
                                .on_press(Message::ConfigInput(ConfigInput::GotenbergUrlSave))
                        ]
                        .into(),
                    ])
                    .spacing(cosmic::theme::spacing().space_xxs),
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
fn cedilla_main_view<'a>(
    app_config: &'a CedillaConfig,
    editor: &'a EditorState,
    preview: &'a MarkdownPreview,
    panes: &'a pane_grid::State<PaneContent>,
    preview_state: &'a PreviewState,
) -> Element<'a, Message> {
    let spacing = theme::active().cosmic().spacing;

    let create_editor = || {
        container(responsive(|size| {
            let highlighter_theme = match app_config.app_theme {
                AppTheme::Dark => highlighter::Theme::Base16Ocean,
                AppTheme::Light => highlighter::Theme::InspiredGitHub,
                AppTheme::System => highlighter::Theme::Base16Ocean,
            };

            widget::id_container(
                scrollable(
                    TextEditor::new(&editor.content)
                        .highlight_with::<highlighter::Highlighter>(
                            highlighter::Settings {
                                theme: highlighter_theme,
                                token: editor
                                    .path
                                    .as_ref()
                                    .and_then(|path| path.extension()?.to_str())
                                    .unwrap_or("md")
                                    .to_string(),
                            },
                            |highlight, _theme| highlight.to_format(),
                        )
                        .size(app_config.text_size)
                        .padding(0)
                        .retain_focus_on_external_click(true)
                        .on_action(Message::Edit),
                )
                .id(editor_scrollable_id())
                .on_scroll(|vp| Message::ScrollChanged(editor_scrollable_id(), vp))
                .height(Length::Fixed(size.height - 5.)), // This is a bit of a workaround but it works,
                widget::Id::new("text_editor_widget"),
            )
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

            let highlighter_theme = match app_config.app_theme {
                AppTheme::Dark => highlighter::Theme::Base16Ocean,
                AppTheme::Light => highlighter::Theme::InspiredGitHub,
                AppTheme::System => highlighter::Theme::Base16Ocean,
            };

            let pane_content: Element<'a, Message> = match content {
                PaneContent::Editor => create_editor().into(),
                PaneContent::Preview => container(
                    scrollable(
                        MarkWidget::new(&preview.markstate)
                            .on_updating_state(Message::UpdateMarkState)
                            .on_clicking_link(Message::LaunchUrl)
                            .text_size(app_config.text_size)
                            .code_highlight_theme(highlighter_theme)
                            .on_drawing_image(|info| {
                                if let Some(image) = preview.images.get(info.url).cloned() {
                                    let mut img = widget::image(image);
                                    if let Some(w) = info.width {
                                        img = img.width(w);
                                    }
                                    if let Some(h) = info.height {
                                        img = img.height(h);
                                    }
                                    img.into()
                                } else if let Some(svg_f) = preview.svgs.get(info.url).cloned() {
                                    let mut svg = widget::svg(svg_f);
                                    if let Some(w) = info.width {
                                        svg = svg.width(w);
                                    }
                                    if let Some(h) = info.height {
                                        svg = svg.height(h);
                                    }
                                    svg.into()
                                } else {
                                    "...".into()
                                }
                            }),
                    )
                    .id(preview_scrollable_id())
                    .on_scroll(|vp| Message::ScrollChanged(preview_scrollable_id(), vp))
                    .width(Length::Fill),
                )
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
        let file_path = match editor.path.as_deref().and_then(Path::to_str) {
            Some(path) => {
                if path.starts_with(&app_config.vault_path) {
                    let relative = path
                        .trim_start_matches(&*app_config.vault_path)
                        .trim_start_matches('/');
                    text(format!("Cedilla Vault/{relative}")).size(12)
                } else {
                    text(path).size(12)
                }
            }
            None => text(fl!("new-file")).size(12),
        };

        let dirty_indicator = if editor.is_dirty {
            text("•").size(12)
        } else {
            text("").size(12)
        };

        let position = {
            let cursor = editor.content.cursor();
            text(format!(
                "{}:{}",
                cursor.position.line + 1,
                cursor.position.column + 1
            ))
            .size(12)
        };

        container(
            row![file_path, dirty_indicator, horizontal(), position]
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
                horizontal().width(18.),
                button::icon(icons::get_handle("helperbar/link-symbolic", 18))
                    .on_press(Message::ApplyFormatting(utils::SelectionAction::Hyperlink)),
                button::icon(icons::get_handle("helperbar/code-symbolic", 18))
                    .on_press(Message::ApplyFormatting(utils::SelectionAction::Code)),
                button::icon(icons::get_handle("helperbar/image-symbolic", 18))
                    .on_press(Message::ApplyFormatting(utils::SelectionAction::Image)),
                horizontal().width(18.),
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
                horizontal(),
                button::icon(icons::get_handle("helperbar/pdf-symbolic", 18)).on_press_maybe(
                    app_config
                        .is_gotenberg_configured()
                        .then_some(Message::ExportPDF)
                )
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

/// Returns the text editor scrollable Id
fn editor_scrollable_id() -> widget::Id {
    widget::Id::new("editor_scroll")
}

/// Returns the cotnent preview scrollable Id
fn preview_scrollable_id() -> widget::Id {
    widget::Id::new("preview_scroll")
}

/// Creates the default panes for the app
fn create_default_panes() -> pane_grid::State<PaneContent> {
    let (mut panes, first_pane) = pane_grid::State::new(PaneContent::Editor);
    panes.split(pane_grid::Axis::Vertical, first_pane, PaneContent::Preview);
    panes
}
