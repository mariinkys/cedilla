// SPDX-License-Identifier: GPL-3.0

use crate::app::core::editor::{EditorScrollState, EditorSearchState, EditorState};
use crate::app::core::history::HistoryState;
use crate::app::core::preview::MarkdownPreview;
use crate::app::core::utils::{self, CedillaToast};
use crate::app::{
    AppModel, Message, PreviewState, State, editor_scrollable_id, preview_scrollable_id,
};
use crate::app::{DiscardChangesAction, create_default_panes};
use crate::config::{BoolState, ShowState};
use cosmic::iced_widget::scrollable::scroll_to;
use cosmic::prelude::*;
use frostmark::MarkState;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use widgets::text_editor;

impl AppModel {
    pub fn handle_startup(&mut self) -> Task<cosmic::Action<Message>> {
        let panes = create_default_panes();

        let preview_state = match self.config.last_preview_showstate {
            ShowState::Show => PreviewState::Shown,
            ShowState::Hide => PreviewState::Hidden,
        };

        let path = match self.config.open_last_file {
            BoolState::Yes => self.config.last_open_file.clone(),
            BoolState::No => None,
        };

        if let Some(p) = path
            && p.exists()
        {
            // store parent directory of selected file
            self.selected_nav_path = p.parent().map(|p| p.to_path_buf());

            return Task::perform(utils::files::load_file(p), |res| {
                cosmic::action::app(Message::OpenFile(res))
            })
            .chain(Task::done(cosmic::action::app(Message::SetPreviewState(
                preview_state,
            ))));
        }

        self.state = State::Ready {
            editor: EditorState {
                path: None,
                content: text_editor::Content::new(),
                is_dirty: true,
                history: HistoryState::default(),
                scroll: EditorScrollState::default(),
                search: EditorSearchState::default(),
                ignore_next_external_change: false,
            },
            preview: MarkdownPreview {
                markstate: MarkState::with_html_and_markdown(""),
                images: HashMap::new(),
                svgs: HashMap::new(),
                images_in_progress: HashSet::new(),
            },
            panes,
            preview_state,
        };
        Task::none()
    }

    pub fn handle_new_file(&mut self) -> Task<cosmic::Action<Message>> {
        let panes = get_previous_pane_state(self);
        let preview_state = get_previous_preview_state(self);

        self.state = State::Ready {
            editor: EditorState {
                path: None,
                content: text_editor::Content::new(),
                is_dirty: true,
                history: HistoryState::default(),
                scroll: EditorScrollState {
                    // pre-absorb the programmatic resets we're about to fire
                    pending_editor_scrolls: 1,
                    pending_preview_scrolls: 1,
                    ..EditorScrollState::default()
                },
                search: EditorSearchState::default(),
                ignore_next_external_change: false,
            },
            preview: MarkdownPreview {
                markstate: MarkState::with_html_and_markdown(""),
                images: HashMap::new(),
                svgs: HashMap::new(),
                images_in_progress: HashSet::new(),
            },
            panes,
            preview_state,
        };

        Task::batch([
            scroll_to(editor_scrollable_id(), crate::app::utils::scroll::abs(0.0))
                .map(cosmic::action::app),
            scroll_to(preview_scrollable_id(), crate::app::utils::scroll::abs(0.0))
                .map(cosmic::action::app),
        ])
    }

    pub fn handle_new_vault_file(&mut self, file_name: String) -> Task<cosmic::Action<Message>> {
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
            return self.handle_add_toast(CedillaToast::new(e));
        }

        self.insert_file_node(&file_path, &dir);

        let panes = get_previous_pane_state(self);
        let preview_state = get_previous_preview_state(self);

        self.state = State::Ready {
            editor: EditorState {
                path: Some(file_path),
                content: text_editor::Content::new(),
                is_dirty: true,
                history: HistoryState::default(),
                scroll: EditorScrollState {
                    // pre-absorb the programmatic resets we're about to fire
                    pending_editor_scrolls: 1,
                    pending_preview_scrolls: 1,
                    ..EditorScrollState::default()
                },
                search: EditorSearchState::default(),
                ignore_next_external_change: false,
            },
            preview: MarkdownPreview {
                markstate: MarkState::with_html_and_markdown(""),
                images: HashMap::new(),
                svgs: HashMap::new(),
                images_in_progress: HashSet::new(),
            },
            panes,
            preview_state,
        };

        Task::batch([
            scroll_to(editor_scrollable_id(), crate::app::utils::scroll::abs(0.0))
                .map(cosmic::action::app),
            scroll_to(preview_scrollable_id(), crate::app::utils::scroll::abs(0.0))
                .map(cosmic::action::app),
        ])
    }

    pub fn handle_new_vault_folder(
        &mut self,
        folder_name: String,
    ) -> Task<cosmic::Action<Message>> {
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

        // create the folder on disk
        if let Err(e) = std::fs::create_dir(&folder_path) {
            return self.handle_add_toast(CedillaToast::new(e));
        }

        // insert folder to navbar
        self.insert_folder_node(&folder_path, &dir);

        Task::none()
    }

    pub fn handle_save_file(&mut self) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        if !editor.is_dirty {
            return Task::none();
        }

        let content = editor.content.text();
        let path = editor.path.clone();
        let vault_path = self.config.vault_path.clone();
        editor.ignore_next_external_change = true;

        Task::perform(
            async move {
                match path {
                    // We're editing an alreaday existing file
                    Some(path) => Some(utils::files::save_file(path, content).await),
                    // We want to save a new file
                    None => match utils::files::open_markdown_file_saver(vault_path).await {
                        Some(path) => Some(utils::files::save_file(path.into(), content).await),
                        // Error selecting where to save the file
                        None => None,
                    },
                }
            },
            |res| cosmic::action::app(Message::FileSaved(res)),
        )
    }

    pub fn handle_open_file(
        &mut self,
        result: Result<(PathBuf, Arc<String>), anywho::Error>,
    ) -> Task<cosmic::Action<Message>> {
        match result {
            Ok((path, content)) => {
                // store parent directory of selected file in nav_path only if path is inside vault
                let vault_path = self.config.vault_path();
                if path.starts_with(&vault_path) {
                    self.selected_nav_path = path.parent().map(|p| p.to_path_buf());
                }

                let panes = get_previous_pane_state(self);
                let preview_state = get_previous_preview_state(self);

                let markstate = MarkState::with_html_and_markdown(content.as_ref());
                let images_in_progress = HashSet::new();

                self.state = State::Ready {
                    editor: EditorState {
                        path: Some(path),
                        content: text_editor::Content::with_text(content.as_ref()),
                        is_dirty: false,
                        history: HistoryState::new_with_content(content.to_string()),
                        scroll: EditorScrollState {
                            // pre-absorb the programmatic resets we're about to fire
                            pending_editor_scrolls: 1,
                            pending_preview_scrolls: 1,
                            ..EditorScrollState::default()
                        },
                        search: EditorSearchState::default(),
                        ignore_next_external_change: false,
                    },
                    preview: MarkdownPreview {
                        markstate,
                        images: HashMap::new(),
                        svgs: HashMap::new(),
                        images_in_progress,
                    },
                    panes,
                    preview_state,
                };

                let reset_editor =
                    scroll_to(editor_scrollable_id(), crate::app::utils::scroll::abs(0.0))
                        .map(cosmic::action::app);

                let reset_preview =
                    scroll_to(preview_scrollable_id(), crate::app::utils::scroll::abs(0.0))
                        .map(cosmic::action::app);

                if let State::Ready {
                    editor, preview, ..
                } = &mut self.state
                {
                    return utils::images::download_images(
                        &mut preview.markstate,
                        &mut preview.images_in_progress,
                        &editor.path,
                    )
                    .chain(reset_editor)
                    .chain(reset_preview);
                }

                Task::none()
            }
            Err(e) => self.handle_add_toast(CedillaToast::new(e)),
        }
    }

    pub fn handle_file_saved(
        &mut self,
        result: Option<Result<PathBuf, anywho::Error>>,
    ) -> Task<cosmic::Action<Message>> {
        if let Some(res) = result {
            match res {
                Ok(new_path) => {
                    let State::Ready { editor, .. } = &mut self.state else {
                        return Task::none();
                    };

                    editor.path = Some(new_path);
                    editor.is_dirty = false;

                    self.handle_add_toast(CedillaToast::new("File Saved!"))
                }
                Err(e) => self.handle_add_toast(CedillaToast::new(e)),
            }
        } else {
            let State::Ready { editor, .. } = &mut self.state else {
                return Task::none();
            };
            editor.ignore_next_external_change = false;

            Task::none()
        }
    }

    pub fn handle_discard_changes(
        &mut self,
        action: DiscardChangesAction,
    ) -> Task<cosmic::Action<Message>> {
        match action {
            DiscardChangesAction::CloseApp => {
                std::process::exit(0);
            }
            DiscardChangesAction::OpenFile(path) => {
                Task::perform(utils::files::load_file(path), |res| {
                    cosmic::action::app(Message::OpenFile(res))
                })
            }
        }
    }

    pub fn handle_external_file_changed(&mut self, path: PathBuf) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        if editor.ignore_next_external_change {
            editor.ignore_next_external_change = false;
            return Task::none();
        }

        let currently_open = editor.path.as_ref() == Some(&path);
        let already_shown = self
            .dialog_pages
            .iter()
            .any(|d| matches!(d, crate::app::dialogs::DialogPage::ExternalFileModified(_)));

        if currently_open && !already_shown {
            editor.is_dirty = true; // we mark it as dirty so if the file has been removed and we keep the current version it creates it again
            self.dialog_pages
                .push_back(crate::app::dialogs::DialogPage::ExternalFileModified(path));
        }

        Task::none()
    }
}

/// Returns the previous state of the panes if available, default if not
fn get_previous_pane_state(
    app_model: &AppModel,
) -> cosmic::widget::pane_grid::State<crate::app::PaneContent> {
    let State::Ready { panes, .. } = &app_model.state else {
        return create_default_panes();
    };

    panes.clone()
}

/// Returns the previous state of the preview if available, default if not
fn get_previous_preview_state(app_model: &AppModel) -> PreviewState {
    let State::Ready { preview_state, .. } = &app_model.state else {
        return PreviewState::Shown;
    };

    preview_state.clone()
}
