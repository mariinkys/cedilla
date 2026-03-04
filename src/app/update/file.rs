// SPDX-License-Identifier: GPL-3.0

use crate::app::core::editor::EditorState;
use crate::app::core::history::HistoryState;
use crate::app::core::preview::MarkdownPreview;
use crate::app::core::utils::{self, CedillaToast};
use crate::app::{AppModel, Message, PreviewState, State};
use crate::app::{DiscardChangesAction, create_default_panes};
use crate::config::{BoolState, ShowState};
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
        let panes = create_default_panes();

        self.state = State::Ready {
            editor: EditorState {
                path: None,
                content: text_editor::Content::new(),
                is_dirty: true,
                history: HistoryState::default(),
            },
            preview: MarkdownPreview {
                markstate: MarkState::with_html_and_markdown(""),
                images: HashMap::new(),
                svgs: HashMap::new(),
                images_in_progress: HashSet::new(),
            },
            panes,
            preview_state: PreviewState::Shown,
        };
        Task::none()
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

        let panes = create_default_panes();

        self.state = State::Ready {
            editor: EditorState {
                path: Some(file_path),
                content: text_editor::Content::new(),
                is_dirty: true,
                history: HistoryState::default(),
            },
            preview: MarkdownPreview {
                markstate: MarkState::with_html_and_markdown(""),
                images: HashMap::new(),
                svgs: HashMap::new(),
                images_in_progress: HashSet::new(),
            },
            panes,
            preview_state: PreviewState::Shown,
        };

        Task::none()
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
            |res| match res {
                Some(result) => cosmic::action::app(Message::FileSaved(result)),
                None => cosmic::action::none(),
            },
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

                let panes = create_default_panes();

                let markstate = MarkState::with_html_and_markdown(content.as_ref());
                let images_in_progress = HashSet::new();

                self.state = State::Ready {
                    editor: EditorState {
                        path: Some(path),
                        content: text_editor::Content::with_text(content.as_ref()),
                        is_dirty: false,
                        history: HistoryState::new_with_content(content.to_string()),
                    },
                    preview: MarkdownPreview {
                        markstate,
                        images: HashMap::new(),
                        svgs: HashMap::new(),
                        images_in_progress,
                    },
                    panes,
                    preview_state: PreviewState::Shown,
                };

                if let State::Ready {
                    editor, preview, ..
                } = &mut self.state
                {
                    return utils::images::download_images(
                        &mut preview.markstate,
                        &mut preview.images_in_progress,
                        &editor.path,
                    );
                }

                Task::none()
            }
            Err(e) => self.handle_add_toast(CedillaToast::new(e)),
        }
    }

    pub fn handle_file_saved(
        &mut self,
        result: Result<PathBuf, anywho::Error>,
    ) -> Task<cosmic::Action<Message>> {
        match result {
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
}
