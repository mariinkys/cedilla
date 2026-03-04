// SPDX-License-Identifier: GPL-3.0

use crate::app::core::utils::{self, CedillaToast};
use crate::app::{AppModel, Message, PreviewState, State};
use crate::config::ShowState;
use cosmic::iced::window;
use cosmic::prelude::*;
use std::process;

impl AppModel {
    pub fn handle_export_pdf(&mut self) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor_content,
            path,
            ..
        } = &mut self.state
        else {
            return Task::none();
        };

        let content = editor_content.text();

        if self.config.is_gotenberg_configured() && !content.trim().is_empty() {
            let client = self.gotenberg_client.clone();
            let file_path = path.clone();

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

    pub fn handle_app_close_requested(
        &mut self,
        window_id: window::Id,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor_content,
            is_dirty,
            history,
            preview_state,
            path,
            ..
        } = &self.state
        else {
            return Task::none();
        };

        if Some(window_id) != self.core.main_window_id() {
            return Task::none();
        }

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

            if let Err(err) = self.config.set_last_open_file(handler, path.clone()) {
                eprintln!("{err}");
            }
        }

        if *is_dirty {
            // if it's a vault path with any modification or if it's a new file with any content
            if (path.is_some() && history.history_index != 0)
                || (path.is_none() && !editor_content.text().trim().is_empty())
            {
                println!("TODO: We're here but for some reason it doesn't work");
                //self.update(Message::DialogAction(
                //    dialogs::DialogAction::OpenConfirmCloseFileDialog(
                //        DiscardChangesAction::CloseApp,
                //    ),
                //))
                process::exit(0);
            } else {
                process::exit(0);
            }
        } else {
            process::exit(0);
        }
    }
}
