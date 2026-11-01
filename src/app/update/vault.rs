// SPDX-License-Identifier: GPL-3.0

use crate::app::core::project::{canonical, remap_path};
use crate::app::core::utils::{self, CedillaToast};
use crate::app::{AppModel, Message, State};
use cosmic::prelude::*;
use std::path::PathBuf;

impl AppModel {
    pub fn handle_delete_node(&mut self, path: PathBuf) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        editor.ignore_next_external_change = true;
        let delete_result = if path.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };

        if let Err(e) = delete_result {
            editor.ignore_next_external_change = false;
            return self.handle_add_toast(CedillaToast::new(e));
        }

        self.forget_nav_paths(&path);
        self.rebuild_nav();

        // if the open file was deleted (directly or inside a deleted folder), start a new empty file
        if let State::Ready { editor, .. } = &self.state
            && editor
                .path
                .as_deref()
                .is_some_and(|p| canonical(p).starts_with(&path))
        {
            return self.handle_new_file();
        }

        Task::none()
    }

    pub fn handle_rename_node(
        &mut self,
        old_path: PathBuf,
        new_name: String,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        let new_name = if old_path.is_dir() || new_name.ends_with(".md") {
            new_name
        } else {
            format!("{}.md", new_name)
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

        self.remap_nav_paths(&old_path, &new_path);

        // update the open editor state if the open file was inside the renamed path
        if let State::Ready { editor, .. } = &mut self.state
            && let Some(current) = editor.path.as_deref()
        {
            editor.path = Some(remap_path(&canonical(current), &old_path, &new_path));
        }

        // rebuild after updating the editor path so the renamed file stays highlighted
        self.rebuild_nav();

        Task::none()
    }

    pub fn handle_move_node(
        &mut self,
        source_path: PathBuf,
        target_path: PathBuf,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        let file_name = match source_path.file_name() {
            Some(n) => n,
            None => return Task::none(),
        };
        let target_path = canonical(&target_path);
        let dest = target_path.join(file_name);

        if source_path == target_path || dest == source_path {
            return Task::none();
        }

        // fs::rename would silently overwrite an existing file
        if dest.exists() {
            return self.handle_add_toast(CedillaToast::new(format!(
                "A file or folder named {:?} already exists there",
                file_name
            )));
        }

        editor.ignore_next_external_change = true;
        if let Err(e) = std::fs::rename(&source_path, &dest) {
            editor.ignore_next_external_change = false;
            return self.handle_add_toast(CedillaToast::new(e));
        }

        self.remap_nav_paths(&source_path, &dest);

        if let State::Ready { editor, .. } = &mut self.state
            && let Some(current) = editor.path.as_deref()
        {
            editor.path = Some(remap_path(&canonical(current), &source_path, &dest));
        }

        // open the destination folder so the user sees where the item went
        self.reveal_in_nav(&dest);
        self.rebuild_nav();

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

                // load the vault at its new location
                self.expanded_folders.clear();
                self.expanded_folders
                    .insert(canonical(&self.config.vault_path()));
                self.selected_nav_path = None;
                self.rebuild_nav();

                Task::done(cosmic::action::app(Message::NewFile))
            }
            Err(e) => self.handle_add_toast(CedillaToast::new(e)),
        }
    }
}
