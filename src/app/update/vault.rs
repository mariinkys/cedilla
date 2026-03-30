// SPDX-License-Identifier: GPL-3.0

use crate::app::core::project::ProjectNode;
use crate::app::core::utils::{self, CedillaToast};
use crate::app::{AppModel, Message, State};
use cosmic::prelude::*;
use cosmic::widget::segmented_button;
use std::path::PathBuf;

impl AppModel {
    pub fn handle_delete_node(
        &mut self,
        entity: segmented_button::Entity,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        let Some(node) = self.nav_model.data::<ProjectNode>(entity).cloned() else {
            return Task::none();
        };

        let path = match &node {
            ProjectNode::File { path, .. } => path.clone(),
            ProjectNode::Folder { path, .. } => path.clone(),
        };

        editor.ignore_next_external_change = true;
        let delete_result = match &node {
            ProjectNode::File { .. } => std::fs::remove_file(&path),
            ProjectNode::Folder { .. } => std::fs::remove_dir_all(&path),
        };

        if let Err(e) = delete_result {
            editor.ignore_next_external_change = false;
            return self.handle_add_toast(CedillaToast::new(e));
        }

        self.remove_nav_node(&path);

        // if the deleted file was currently open, create a new empty file
        if let State::Ready { editor, .. } = &self.state
            && editor.path.as_deref() == Some(&path)
        {
            return self.handle_new_file();
        }

        Task::none()
    }

    pub fn handle_rename_node(
        &mut self,
        entity: segmented_button::Entity,
        new_name: String,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        let Some(node) = self.nav_model.data::<ProjectNode>(entity).cloned() else {
            return Task::none();
        };

        let old_path = match &node {
            ProjectNode::File { path, .. } | ProjectNode::Folder { path, .. } => path.clone(),
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
            return self.handle_add_toast(CedillaToast::new(format!(
                "A file or folder named {:?} already exists",
                new_name
            )));
        }

        editor.ignore_next_external_change = true;
        if let Err(e) = std::fs::rename(&old_path, &new_path) {
            editor.ignore_next_external_change = false;
            return self.handle_add_toast(CedillaToast::new(e));
        }

        self.rename_nav_node(&old_path, &new_path, &new_name);

        // update the open editor state if the open file was inside the renamed path
        #[allow(clippy::collapsible_if)]
        if let State::Ready { editor, .. } = &mut self.state {
            if let Some(current) = editor.path.as_deref() {
                if current.starts_with(&old_path) {
                    let suffix = current.strip_prefix(&old_path).unwrap().to_path_buf();
                    editor.path = Some(if suffix == std::path::Path::new("") {
                        new_path.clone()
                    } else {
                        new_path.join(suffix)
                    });
                }
            }
        }

        Task::none()
    }

    pub fn handle_move_node(
        &mut self,
        source_entity: segmented_button::Entity,
        target_path: PathBuf,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        let source_path = match self.nav_model.data::<ProjectNode>(source_entity) {
            Some(ProjectNode::File { path, .. } | ProjectNode::Folder { path, .. }) => path.clone(),
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

        editor.ignore_next_external_change = true;
        if let Err(e) = std::fs::rename(&source_path, &dest) {
            editor.ignore_next_external_change = false;
            return self.handle_add_toast(CedillaToast::new(e));
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
            let vault_path = self.config.vault_path();
            self.nav_model.clear();
            self.open_vault_folder(&vault_path);
        }

        if let State::Ready { editor, .. } = &mut self.state
            && editor.path.as_deref() == Some(&source_path)
        {
            editor.path = Some(dest);
        }

        Task::none()
    }

    pub fn handle_move_vault(&mut self) -> Task<cosmic::Action<Message>> {
        let old_vault_path = self.config.vault_path.clone();

        Task::perform(
            async move {
                match utils::files::open_folder_picker(old_vault_path.clone()).await {
                    Some(path) => {
                        Some(utils::files::move_vault(path.into(), old_vault_path.into()).await)
                    }
                    // Error selecting where to move the vault
                    None => None,
                }
            },
            |res| match res {
                Some(result) => cosmic::action::app(Message::VaultMoved(result)),
                None => cosmic::action::none(),
            },
        )
    }

    pub fn handle_vault_moved(
        &mut self,
        result: Result<PathBuf, anywho::Error>,
    ) -> Task<cosmic::Action<Message>> {
        match result {
            Ok(new_path) => {
                // update the vault path in the config
                #[allow(clippy::collapsible_if)]
                if let Some(handler) = &self.config_handler {
                    if let Err(err) = self
                        .config
                        .set_vault_path(handler, new_path.to_string_lossy().to_string())
                    {
                        eprintln!("{err}");
                        // even if it fails we update the config (it won't get saved after restart)
                        let mut old_config = self.config.clone();
                        old_config.vault_path = new_path.to_string_lossy().to_string();
                        self.config = old_config;
                    }
                }

                // clear the navbar
                self.core.nav_bar_set_toggled(false);
                self.nav_model.clear();

                // load vault
                self.open_vault_folder(self.config.vault_path());

                Task::done(cosmic::action::app(Message::NewFile))
            }
            Err(e) => self.handle_add_toast(CedillaToast::new(e)),
        }
    }
}
