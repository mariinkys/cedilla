// SPDX-License-Identifier: GPL-3.0

use crate::app::app_menu::MenuAction;
use crate::app::context_page::ContextPage;
use crate::app::core::utils;
use crate::app::{AppModel, Message, PreviewState};
use crate::app::{State, dialogs};
use cosmic::prelude::*;

impl AppModel {
    pub fn handle_menu_action(&mut self, action: MenuAction) -> Task<cosmic::Action<Message>> {
        let State::Ready { preview_state, .. } = &mut self.state else {
            return Task::none();
        };

        match action {
            MenuAction::About => self.handle_toggle_context_page(ContextPage::About),
            MenuAction::Settings => self.handle_toggle_context_page(ContextPage::Settings),
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
            MenuAction::NewFile => self.handle_new_file(),
            MenuAction::NewVaultFile => {
                self.handle_dialog_action(dialogs::DialogAction::OpenNewVaultFileDialog)
            }
            MenuAction::NewVaultFolder => {
                self.handle_dialog_action(dialogs::DialogAction::OpenNewVaultFolderDialog)
            }
            MenuAction::SaveFile => self.handle_save_file(),
            MenuAction::TogglePreview => {
                match preview_state {
                    PreviewState::Hidden => *preview_state = PreviewState::Shown,
                    PreviewState::Shown => *preview_state = PreviewState::Hidden,
                }
                Task::none()
            }
            MenuAction::Undo => self.handle_undo(),
            MenuAction::Redo => self.handle_redo(),
            MenuAction::Search => self.handle_search(utils::search::SearchAction::ToggleSearch),
        }
    }
}
