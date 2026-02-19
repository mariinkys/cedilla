// SPDX-License-Identifier: GPL-3.0-only
#![allow(clippy::collapsible_if)]

// Code based on System76's, see: https://github.com/pop-os/cosmic-edit/blob/master/src/project.rs

use cosmic::widget::icon;

use std::{
    cmp::Ordering,
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    app::AppModel,
    icons::{self},
};

impl AppModel {
    pub fn open_vault_folder<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref();

        if let Ok(mut node) = ProjectNode::new(path) {
            if let ProjectNode::Folder { open, root, .. } = &mut node {
                *open = true;
                *root = true;
            }
            let id = self
                .nav_model
                .insert()
                .icon(node.icon(18))
                .text("Cedilla Vault")
                .data(node)
                .id();

            let position = self.nav_model.position(id).unwrap_or(0);
            self.open_folder(path, position + 1, 1);
        }
    }

    pub fn open_folder<P: AsRef<Path>>(&mut self, path: P, mut position: u16, indent: u16) {
        let mut nodes = Vec::new();
        for entry_res in ignore::WalkBuilder::new(&path)
            .hidden(false)
            .max_depth(Some(1))
            .build()
        {
            let entry = match entry_res {
                Ok(ok) => ok,
                Err(_) => continue,
            };
            if entry.depth() == 0 {
                continue;
            }
            let node = match ProjectNode::new(entry.path()) {
                Ok(ok) => ok,
                Err(_) => continue,
            };
            nodes.push(node);
        }

        nodes.sort();

        for node in nodes {
            self.nav_model
                .insert()
                .position(position)
                .indent(indent)
                .icon(node.icon(18))
                .text(node.name().to_string())
                .data(node);
            position += 1;
        }
    }

    pub fn insert_file_node(&mut self, file_path: &PathBuf, parent_dir: &PathBuf) {
        let Ok(node) = ProjectNode::new(file_path) else {
            return;
        };

        let (insert_position, insert_indent) = {
            let mut pos = 0u16;
            let mut indent = 1u16;
            for nav_id in self.nav_model.iter() {
                if let Some(ProjectNode::Folder { path, .. }) =
                    self.nav_model.data::<ProjectNode>(nav_id)
                {
                    if *path == *parent_dir {
                        let folder_pos = self.nav_model.position(nav_id).unwrap_or(0);
                        let folder_indent = self.nav_model.indent(nav_id).unwrap_or(0);

                        let children: Vec<(u16, u16)> = self
                            .nav_model
                            .iter()
                            .filter_map(|child_id| {
                                let child_pos = self.nav_model.position(child_id)?;
                                let child_indent = self.nav_model.indent(child_id)?;
                                Some((child_pos, child_indent))
                            })
                            .collect();

                        let mut insert_at = folder_pos + 1;
                        for (child_pos, child_indent) in &children {
                            if *child_pos == insert_at && *child_indent > folder_indent {
                                insert_at += 1;
                            }
                        }

                        pos = insert_at;
                        indent = folder_indent + 1;
                        break;
                    }
                }
            }
            (pos, indent)
        };

        self.nav_model
            .insert()
            .position(insert_position)
            .indent(insert_indent)
            .icon(node.icon(18))
            .text(node.name().to_string())
            .data(node);
    }

    pub fn insert_folder_node(&mut self, folder_path: &PathBuf, parent_dir: &PathBuf) {
        let Ok(node) = ProjectNode::new(folder_path) else {
            return;
        };

        let (insert_position, insert_indent) = {
            let mut pos = 0u16;
            let mut indent = 1u16;
            for nav_id in self.nav_model.iter() {
                if let Some(ProjectNode::Folder { path, .. }) =
                    self.nav_model.data::<ProjectNode>(nav_id)
                {
                    if *path == *parent_dir {
                        let folder_pos = self.nav_model.position(nav_id).unwrap_or(0);
                        let folder_indent = self.nav_model.indent(nav_id).unwrap_or(0);

                        let children: Vec<(u16, u16, bool)> = self
                            .nav_model
                            .iter()
                            .filter_map(|child_id| {
                                let child_pos = self.nav_model.position(child_id)?;
                                let child_indent = self.nav_model.indent(child_id)?;
                                let is_file = matches!(
                                    self.nav_model.data::<ProjectNode>(child_id),
                                    Some(ProjectNode::File { .. })
                                );
                                Some((child_pos, child_indent, is_file))
                            })
                            .collect();

                        let mut insert_at = folder_pos + 1;
                        for (child_pos, child_indent, is_file) in &children {
                            if *child_pos == insert_at && *child_indent > folder_indent {
                                if *is_file {
                                    break;
                                }
                                insert_at += 1;
                            }
                        }

                        pos = insert_at;
                        indent = folder_indent + 1;
                        break;
                    }
                }
            }
            (pos, indent)
        };

        self.nav_model
            .insert()
            .position(insert_position)
            .indent(insert_indent)
            .icon(node.icon(16))
            .text(node.name().to_string())
            .data(node);
    }

    pub fn remove_nav_node(&mut self, target_path: &PathBuf) {
        let entity_opt =
            self.nav_model
                .iter()
                .find(|&id| match self.nav_model.data::<ProjectNode>(id) {
                    Some(ProjectNode::File { path, .. }) => path == target_path,
                    Some(ProjectNode::Folder { path, .. }) => path == target_path,
                    None => false,
                });

        let Some(entity) = entity_opt else { return };

        let position = self.nav_model.position(entity).unwrap_or(0);
        let indent = self.nav_model.indent(entity).unwrap_or(0);

        // remove all children (if it's a folder)
        while let Some(child) = self.nav_model.entity_at(position + 1) {
            if self.nav_model.indent(child).unwrap_or(0) > indent {
                self.nav_model.remove(child);
            } else {
                break;
            }
        }

        // remove the node itself
        self.nav_model.remove(entity);

        // clear selected path if it was inside the deleted path
        if let Some(selected) = &self.selected_nav_path {
            if selected.starts_with(target_path) || selected == target_path {
                self.selected_nav_path = None;
            }
        }
    }

    pub fn rename_nav_node(&mut self, old_path: &Path, new_path: &Path, new_name: &str) {
        let ids: Vec<_> = self.nav_model.iter().collect();

        for child_id in ids {
            let is_renamed_node = if let Some(child_node) =
                self.nav_model.data_mut::<ProjectNode>(child_id)
            {
                match child_node {
                    ProjectNode::File { path, name } | ProjectNode::Folder { path, name, .. } => {
                        if path.starts_with(old_path) {
                            let suffix = path.strip_prefix(old_path).unwrap().to_path_buf();
                            *path = if suffix == std::path::Path::new("") {
                                new_path.to_path_buf()
                            } else {
                                new_path.join(&suffix)
                            };
                            if suffix == std::path::Path::new("") {
                                *name = new_name.to_string();
                                true
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    }
                }
            } else {
                false
            };
            if is_renamed_node {
                self.nav_model.text_set(child_id, new_name.to_string());
            }
        }

        if let Some(selected) = &self.selected_nav_path {
            if selected.starts_with(old_path) {
                let suffix = selected.strip_prefix(old_path).unwrap().to_path_buf();
                self.selected_nav_path = Some(if suffix == std::path::Path::new("") {
                    new_path.to_path_buf()
                } else {
                    new_path.join(suffix)
                });
            }
        }
    }

    pub fn move_nav_node(
        &mut self,
        source_entity: cosmic::widget::segmented_button::Entity,
        target_entity: cosmic::widget::segmented_button::Entity,
        new_path: &PathBuf,
    ) {
        let source_indent = self.nav_model.indent(source_entity).unwrap_or(0);
        let source_position = self.nav_model.position(source_entity).unwrap_or(0);

        // collect children by walking positions sequentially after source
        let children: Vec<cosmic::widget::segmented_button::Entity> = {
            let mut result = Vec::new();
            let mut pos = source_position + 1;
            while let Some(id) = self.nav_model.entity_at(pos) {
                if self.nav_model.indent(id).unwrap_or(0) > source_indent {
                    result.push(id);
                    pos += 1;
                } else {
                    break;
                }
            }
            result
        };

        for child in children.iter().rev() {
            self.nav_model.remove(*child);
        }
        self.nav_model.remove(source_entity);

        // re-read target position/indent after removals since positions shifted
        let target_position = self.nav_model.position(target_entity).unwrap_or(0);
        let target_indent = self.nav_model.indent(target_entity).unwrap_or(0);

        let target_is_open = matches!(
            self.nav_model.data::<ProjectNode>(target_entity),
            Some(ProjectNode::Folder { open: true, .. })
        );

        // if target is closed, don't insert anything — on_nav_select will
        // populate it correctly from disk when the user opens it
        if !target_is_open {
            return;
        }

        // find insertion point after all existing children of target
        let insert_position = {
            let mut pos = target_position + 1;
            let mut check = target_position + 1;
            while let Some(id) = self.nav_model.entity_at(check) {
                if self.nav_model.indent(id).unwrap_or(0) > target_indent {
                    let child_is_folder = matches!(
                        self.nav_model.data::<ProjectNode>(id),
                        Some(ProjectNode::Folder { .. })
                    );
                    let source_is_folder = new_path.is_dir();
                    if source_is_folder {
                        if child_is_folder {
                            pos = check + 1;
                        }
                    } else {
                        pos = check + 1;
                    }
                    check += 1;
                } else {
                    break;
                }
            }
            pos
        };

        let Ok(node) = ProjectNode::new(new_path) else {
            return;
        };

        self.nav_model
            .insert()
            .position(insert_position)
            .indent(target_indent + 1)
            .icon(node.icon(18))
            .text(node.name().to_string())
            .data(node);

        // populate children only if source was a folder (target is already confirmed open)
        if new_path.is_dir() {
            self.open_folder(new_path, insert_position + 1, target_indent + 2);
        }
    }

    pub fn selected_directory(&self) -> PathBuf {
        self.selected_nav_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(&self.config.vault_path))
    }

    pub fn collect_all_folders(
        &self,
        vault_path: &PathBuf,
        exclude_entity: cosmic::widget::segmented_button::Entity,
    ) -> Vec<(PathBuf, String, u16)> {
        let exclude_path = self
            .nav_model
            .data::<ProjectNode>(exclude_entity)
            .map(|n| match n {
                ProjectNode::Folder { path, .. } | ProjectNode::File { path, .. } => path.clone(),
            });

        let mut result = Vec::new();

        // root always first
        result.push((
            vault_path.clone(),
            self.nav_model
                .iter()
                .find_map(|id| {
                    if matches!(
                        self.nav_model.data::<ProjectNode>(id),
                        Some(ProjectNode::Folder { root: true, .. })
                    ) {
                        self.nav_model.text(id).map(|t| t.to_string())
                    } else {
                        None
                    }
                })
                .unwrap_or("Vault".to_string()),
            0u16,
        ));

        self.collect_folders_recursive(vault_path, 1, &exclude_path, &mut result);
        result
    }

    fn collect_folders_recursive(
        &self,
        dir: &PathBuf,
        indent: u16,
        exclude_path: &Option<PathBuf>,
        result: &mut Vec<(PathBuf, String, u16)>,
    ) {
        let mut nodes: Vec<PathBuf> = ignore::WalkBuilder::new(dir)
            .hidden(false)
            .max_depth(Some(1))
            .build()
            .filter_map(|e| e.ok())
            .filter(|e| e.depth() == 1 && e.path().is_dir())
            .map(|e| e.path().to_path_buf())
            .collect();

        nodes.sort_by(|a, b| {
            a.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .cmp(b.file_name().and_then(|n| n.to_str()).unwrap_or(""))
        });

        for subdir in nodes {
            if let Some(excl) = exclude_path {
                if subdir == *excl || subdir.starts_with(excl) {
                    continue;
                }
            }
            let name = subdir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();
            result.push((subdir.clone(), name, indent - 1));
            self.collect_folders_recursive(&subdir, indent + 1, exclude_path, result);
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectNode {
    Folder {
        name: String,
        path: PathBuf,
        open: bool,
        root: bool,
    },
    File {
        name: String,
        path: PathBuf,
    },
}

impl ProjectNode {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path = fs::canonicalize(path)?;
        let name = path
            .file_name()
            .ok_or(io::Error::other(format!(
                "path {:?} has no file name",
                path
            )))?
            .to_str()
            .ok_or(io::Error::other(format!(
                "path {:?} is not valid UTF-8",
                path
            )))?
            .to_string();
        Ok(if path.is_dir() {
            Self::Folder {
                path,
                name,
                open: false,
                root: false,
            }
        } else {
            Self::File { path, name }
        })
    }

    pub fn icon(&self, size: u16) -> icon::Icon {
        match self {
            Self::Folder { open, .. } => {
                if *open {
                    icons::get_icon("go-down-symbolic", size)
                } else {
                    icons::get_icon("go-next-symbolic", size)
                }
            }
            Self::File { .. } => icons::get_icon("markdown-symbolic", size),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::Folder { name, .. } => name,
            Self::File { name, .. } => name,
        }
    }
}

impl Ord for ProjectNode {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Folder { .. }, Self::File { .. }) => Ordering::Less,
            (Self::File { .. }, Self::Folder { .. }) => Ordering::Greater,
            _ => self.name().cmp(other.name()),
        }
    }
}

impl PartialOrd for ProjectNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
