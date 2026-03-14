// SPDX-License-Identifier: GPL-3.0

use crate::{app::Message, fl};
use cosmic::widget::menu::{self, items, root, Item, KeyBind, MenuBar, Tree};
use cosmic::Element;
use std::collections::HashMap;

/// Represents a Action that executes after clicking on the application Menu
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    /// Open the About [`ContextPage`] of the application
    About,
    /// Open the Settings [`ContextPage`] of the application
    Settings,
    /// Open a FileDialog to pick a new file to open
    OpenFile,
    /// Create a new empty file
    NewFile,
    /// Create a new vault file
    NewVaultFile,
    /// Create a new vault folder
    NewVaultFolder,
    /// Save the current file
    SaveFile,
    /// Toggle the preview for the current file
    TogglePreview,
    /// Undo
    Undo,
    /// Redo
    Redo,
}

impl menu::action::MenuAction for MenuAction {
    type Message = crate::app::Message;

    fn message(&self) -> Self::Message {
        match self {
            MenuAction::About => Message::MenuAction(MenuAction::About),
            MenuAction::Settings => Message::MenuAction(MenuAction::Settings),
            MenuAction::OpenFile => Message::MenuAction(MenuAction::OpenFile),
            MenuAction::NewFile => Message::MenuAction(MenuAction::NewFile),
            MenuAction::NewVaultFile => Message::MenuAction(MenuAction::NewVaultFile),
            MenuAction::NewVaultFolder => Message::MenuAction(MenuAction::NewVaultFolder),
            MenuAction::SaveFile => Message::MenuAction(MenuAction::SaveFile),
            MenuAction::TogglePreview => Message::MenuAction(MenuAction::TogglePreview),
            MenuAction::Undo => Message::MenuAction(MenuAction::Undo),
            MenuAction::Redo => Message::MenuAction(MenuAction::Redo),
        }
    }
}

pub fn menu_bar<'a>(key_binds: &HashMap<KeyBind, MenuAction>) -> Element<'a, Message> {
    MenuBar::new(vec![
        Tree::with_children(
            Element::from(root(fl!("file"))),
            items(
                key_binds,
                vec![
                    Item::Button(fl!("new-vault-file"), None, MenuAction::NewVaultFile),
                    Item::Button(fl!("new-folder"), None, MenuAction::NewVaultFolder),
                    Item::Button(fl!("open-file"), None, MenuAction::OpenFile),
                    Item::Button(fl!("save-file"), None, MenuAction::SaveFile),
                    Item::Divider,
                    Item::Button(fl!("new-file"), None, MenuAction::NewFile),
                ],
            ),
        ),
        Tree::with_children(
            Element::from(root(fl!("edit"))),
            items(
                key_binds,
                vec![
                    Item::Button(fl!("undo"), None, MenuAction::Undo),
                    Item::Button(fl!("redo"), None, MenuAction::Redo),
                ],
            ),
        ),
        Tree::with_children(
            Element::from(root(fl!("view"))),
            items(
                key_binds,
                vec![
                    Item::Button(fl!("about"), None, MenuAction::About),
                    Item::Button(fl!("settings"), None, MenuAction::Settings),
                ],
            ),
        ),
    ])
    .into()
}
