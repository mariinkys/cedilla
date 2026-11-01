// SPDX-License-Identifier: GPL-3.0

use crate::app::core::project::ProjectNode;
use crate::app::dialogs;
use crate::app::{AppModel, Message, NavMenuAction, State};
use cosmic::prelude::*;
use cosmic::widget::segmented_button;
use std::path::{Path, PathBuf};

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
        // remember the path now: if the navbar gets rebuilt before a menu item is chosen,
        // the entity in that menu item is no longer valid
        self.nav_context_path = self.nav_node_path(entity);
        Task::none()
    }

    pub fn handle_nav_menu_action(
        &mut self,
        action: NavMenuAction,
    ) -> Task<cosmic::Action<Message>> {
        let entity = match action {
            NavMenuAction::DeleteNode(e)
            | NavMenuAction::RenameNode(e)
            | NavMenuAction::MoveNode(e)
            | NavMenuAction::OpenNodeFileManager(e)
            | NavMenuAction::OpenFolderCreationDialog(e)
            | NavMenuAction::OpenVaultFileCreationDialog(e) => e,
        };

        let Some(path) = self
            .nav_node_path(entity)
            .or_else(|| self.nav_context_path.take())
        else {
            return Task::none();
        };
        self.nav_context_path = None;

        // creation dialogs and "open in file manager" act on the folder (a file's parent)
        let folder = if path.is_dir() {
            path.clone()
        } else {
            // a file should always have a parent
            path.parent()
                .map(Path::to_path_buf)
                .unwrap_or_else(|| path.clone())
        };

        match action {
            NavMenuAction::DeleteNode(_) => {
                self.handle_dialog_action(dialogs::DialogAction::OpenDeleteNodeDialog(path))
            }
            NavMenuAction::RenameNode(_) => {
                self.handle_dialog_action(dialogs::DialogAction::OpenRenameNodeDialog(path))
            }
            NavMenuAction::MoveNode(_) => {
                let vault_path = self.config.vault_path();
                self.dialog_state.available_folders = self.collect_all_folders(&vault_path, &path);

                self.handle_dialog_action(dialogs::DialogAction::OpenMoveNodeDialog(path))
            }
            NavMenuAction::OpenNodeFileManager(_) => self.handle_open_in_file_explorer(&folder),
            NavMenuAction::OpenFolderCreationDialog(_) => {
                // the context menu click doesn't go through on_nav_select, so set the target here
                self.selected_nav_path = Some(folder);
                self.handle_dialog_action(dialogs::DialogAction::OpenNewVaultFolderDialog)
            }
            NavMenuAction::OpenVaultFileCreationDialog(_) => {
                self.selected_nav_path = Some(folder);
                self.handle_dialog_action(dialogs::DialogAction::OpenNewVaultFileDialog)
            }
        }
    }

    /// Path of a navbar entry, `None` if the entity no longer exists.
    fn nav_node_path(&self, entity: segmented_button::Entity) -> Option<PathBuf> {
        match self.nav_model.data::<ProjectNode>(entity)? {
            ProjectNode::File { path, .. } | ProjectNode::Folder { path, .. } => Some(path.clone()),
        }
    }
}
