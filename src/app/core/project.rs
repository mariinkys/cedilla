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
    app::{AppModel, State},
    icons::{self},
};

impl AppModel {
    /// Rebuilds the whole navbar from disk.
    ///
    /// Folders in `expanded_folders` are shown open, everything is sorted (folders first),
    /// and the currently open file gets highlighted. Every operation that changes the tree
    /// updates the state and calls this, so the navbar can never drift from what's on disk.
    pub fn rebuild_nav(&mut self) {
        let vault_path = canonical(&self.config.vault_path());

        self.nav_model.clear();

        let Ok(mut root) = ProjectNode::new(&vault_path) else {
            return;
        };

        let root_open = self.expanded_folders.contains(&vault_path);
        if let ProjectNode::Folder { open, root, .. } = &mut root {
            *open = root_open;
            *root = true;
        }

        self.nav_model
            .insert()
            .indent(0)
            .icon(root.icon(18))
            .text("Cedilla Vault")
            .data(root);

        if root_open {
            self.insert_folder_children(&vault_path, 1);
        }

        self.sync_nav_selection();
    }

    /// Appends the (sorted) children of `dir`, recursing into expanded folders.
    fn insert_folder_children(&mut self, dir: &Path, indent: u16) {
        for mut node in read_folder(dir) {
            let expanded_path = match &mut node {
                ProjectNode::Folder { path, open, .. } => {
                    *open = self.expanded_folders.contains(path);
                    open.then(|| path.clone())
                }
                ProjectNode::File { .. } => None,
            };

            self.nav_model
                .insert()
                .indent(indent)
                .icon(node.icon(18))
                .text(node.name().to_string())
                .data(node);

            if let Some(path) = expanded_path {
                self.insert_folder_children(&path, indent + 1);
            }
        }
    }

    /// Highlights the navbar entry of the file open in the editor (or nothing).
    pub fn sync_nav_selection(&mut self) {
        let open_file = match &self.state {
            State::Ready { editor, .. } => editor.path.as_deref().map(canonical),
            _ => None,
        };

        let entity = open_file.and_then(|open_file| {
            self.nav_model.iter().find(|&id| {
                matches!(
                    self.nav_model.data::<ProjectNode>(id),
                    Some(ProjectNode::File { path, .. }) if *path == open_file
                )
            })
        });

        match entity {
            Some(id) => self.nav_model.activate(id),
            None => self.nav_model.deactivate(),
        }
    }

    /// Expands every folder between the vault root and `path` so `path` is visible.
    /// Does nothing for paths outside the vault. Call `rebuild_nav` afterwards.
    pub fn reveal_in_nav(&mut self, path: &Path) {
        let vault_path = canonical(&self.config.vault_path());
        let path = canonical(path);

        if !path.starts_with(&vault_path) {
            return;
        }

        for ancestor in path.ancestors().skip(1) {
            self.expanded_folders.insert(ancestor.to_path_buf());
            if ancestor == vault_path {
                break;
            }
        }
    }

    /// Updates navbar state after `old` was renamed or moved to `new`.
    pub fn remap_nav_paths(&mut self, old: &Path, new: &Path) {
        self.expanded_folders = self
            .expanded_folders
            .drain()
            .map(|p| remap_path(&p, old, new))
            .collect();

        if let Some(selected) = &self.selected_nav_path {
            self.selected_nav_path = Some(remap_path(selected, old, new));
        }
    }

    /// Updates navbar state after `removed` was deleted.
    pub fn forget_nav_paths(&mut self, removed: &Path) {
        self.expanded_folders.retain(|p| !p.starts_with(removed));

        if let Some(selected) = &self.selected_nav_path {
            if selected.starts_with(removed) {
                self.selected_nav_path = removed.parent().map(Path::to_path_buf);
            }
        }
    }

    pub fn selected_directory(&self) -> PathBuf {
        self.selected_nav_path
            .clone()
            .unwrap_or_else(|| self.config.vault_path())
    }

    pub fn collect_all_folders(
        &self,
        vault_path: &PathBuf,
        exclude_path: &Path,
    ) -> Vec<(PathBuf, String, u16)> {
        let exclude_path = Some(exclude_path.to_path_buf());

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

/// Canonical form of `path`, or `path` itself if it can't be resolved (e.g. it was deleted).
pub fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// If `path` is `old` or lives inside it, returns the equivalent path under `new`.
pub fn remap_path(path: &Path, old: &Path, new: &Path) -> PathBuf {
    match path.strip_prefix(old) {
        Ok(suffix) if suffix.as_os_str().is_empty() => new.to_path_buf(),
        Ok(suffix) => new.join(suffix),
        Err(_) => path.to_path_buf(),
    }
}

/// Reads the direct children of `dir` that belong in the navbar (folders and .md/.txt files),
/// sorted with folders first.
fn read_folder(dir: &Path) -> Vec<ProjectNode> {
    let mut nodes: Vec<ProjectNode> = ignore::WalkBuilder::new(dir)
        .hidden(false)
        .max_depth(Some(1))
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.depth() == 1)
        .filter(|entry| {
            !entry.file_type().is_some_and(|ft| ft.is_file())
                || matches!(
                    entry.path().extension().and_then(std::ffi::OsStr::to_str),
                    Some("md" | "txt")
                )
        })
        .filter_map(|entry| ProjectNode::new(entry.path()).ok())
        .collect();

    nodes.sort();
    nodes
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
        match self {
            // Folders are always before files
            Self::Folder { .. } => {
                if let Self::File { .. } = other {
                    return Ordering::Less;
                }
            }
            // Files are always after folders
            Self::File { .. } => {
                if let Self::Folder { .. } = other {
                    return Ordering::Greater;
                }
            }
        }
        crate::i18n::LANGUAGE_SORTER.compare(self.name(), other.name())
    }
}

impl PartialOrd for ProjectNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
