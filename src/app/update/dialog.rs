// SPDX-License-Identifier: GPL-3.0

use crate::app::dialogs;
use crate::app::{AppModel, Message, NavMenuAction, State};
use cosmic::prelude::*;
use cosmic::widget::segmented_button;
use slotmap::Key as SlotmapKey;

impl AppModel {
    pub fn handle_dialog_action(
        &mut self,
        action: dialogs::DialogAction,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { .. } = &mut self.state else {
            return Task::none();
        };

        action.execute(&mut self.dialog_pages, &self.dialog_state)
    }

    pub fn handle_nav_bar_context(
        &mut self,
        entity: segmented_button::Entity,
    ) -> Task<cosmic::Action<Message>> {
        self.nav_bar_context_id = entity;
        Task::none()
    }

    pub fn handle_nav_menu_action(
        &mut self,
        action: NavMenuAction,
    ) -> Task<cosmic::Action<Message>> {
        self.nav_bar_context_id = segmented_button::Entity::null();

        match action {
            NavMenuAction::DeleteNode(entity) => {
                self.handle_dialog_action(dialogs::DialogAction::OpenDeleteNodeDialog(entity))
            }
            NavMenuAction::RenameNode(entity) => {
                self.handle_dialog_action(dialogs::DialogAction::OpenRenameNodeDialog(entity))
            }
            NavMenuAction::MoveNode(entity) => {
                let vault_path = self.config.vault_path();
                self.dialog_state.available_folders = self.collect_all_folders(&vault_path, entity);

                self.handle_dialog_action(dialogs::DialogAction::OpenMoveNodeDialog(entity))
            }
            NavMenuAction::OpenNodeFileManager(entity) => {
                if let Some(entity) = self
                    .nav_model
                    .data::<crate::app::core::project::ProjectNode>(entity)
                {
                    let path = match entity {
                        crate::app::core::project::ProjectNode::Folder { path, .. } => path,
                        crate::app::core::project::ProjectNode::File { path, .. } => {
                            // a file should always have a parent
                            path.parent().unwrap()
                        }
                    };

                    match open::that(path) {
                        Ok(()) => Task::none(),
                        Err(err) => {
                            eprintln!("failed to open {path:?}: {err}");
                            Task::none()
                        }
                    }
                } else {
                    Task::none()
                }
            }
        }
    }
}
