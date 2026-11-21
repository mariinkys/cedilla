// SPDX-License-Identifier: GPL-3.0

use crate::app::core::utils::{self, CedillaToast};
use crate::app::{AppModel, DiscardChangesAction, Message, PreviewState, State, dialogs};
use crate::config::ShowState;
use cosmic::prelude::*;
use std::path::Path;
use std::process;

impl AppModel {
    /// Opens the given path in the default file explorer (or default app if target is not a directory)
    pub fn handle_open_in_file_explorer(&self, path: &Path) -> Task<cosmic::Action<Message>> {
        match open::that(path) {
            Ok(()) => Task::none(),
            Err(err) => {
                eprintln!("failed to open {path:?}: {err}");
                Task::none()
            }
        }
    }

    pub fn handle_export_pdf(&mut self) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        let content = editor.content.text();

        if self.config.is_gotenberg_configured() && !content.trim().is_empty() {
            let client = self.gotenberg_client.clone();
            let file_path = editor.path.clone();

            Task::perform(
                async move {
                    match utils::files::open_pdf_file_saver().await {
                        Some(path) => {
                            Some(utils::pdf::export_pdf(client, file_path, content, path).await)
                        }
                        // Error selecting where to save the file
                        None => None,
                    }
                },
                |res| match res {
                    Some(result) => match result {
                        Ok(_) => cosmic::action::app(Message::AddToast(CedillaToast::new(
                            "Exported Correctly",
                        ))),
                        Err(e) => cosmic::action::app(Message::AddToast(CedillaToast::new(e))),
                    },
                    None => cosmic::action::none(),
                },
            )
        } else {
            Task::none()
        }
    }

    pub fn handle_app_close_requested(&mut self) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor,
            preview_state,
            ..
        } = &self.state
        else {
            return Task::none();
        };

        if let Some(handler) = &self.config_handler {
            let current_preview_state = match preview_state {
                PreviewState::Hidden => ShowState::Hide,
                PreviewState::Shown => ShowState::Show,
            };

            let current_nav_state = match self.core.nav_bar_active() {
                true => ShowState::Show,
                false => ShowState::Hide,
            };

            if let Err(err) = self
                .config
                .set_last_preview_showstate(handler, current_preview_state)
            {
                eprintln!("{err}");
            }

            if let Err(err) = self
                .config
                .set_last_navbar_showstate(handler, current_nav_state)
            {
                eprintln!("{err}");
            }

            if let Err(err) = self.config.set_last_open_file(handler, editor.path.clone()) {
                eprintln!("{err}");
            }
        }

        if editor.is_dirty {
            // if it's a vault path with any modification or if it's a new file with any content
            if editor.needs_confirmation() {
                self.handle_dialog_action(dialogs::DialogAction::OpenConfirmCloseFileDialog(
                    DiscardChangesAction::CloseApp,
                ))
            } else {
                process::exit(0);
            }
        } else {
            process::exit(0);
        }
    }
}
