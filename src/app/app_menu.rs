// SPDX-License-Identifier: GPL-3.0

use crate::{app::Message, fl};
use cosmic::widget::menu::{self, ItemHeight, ItemWidth, KeyBind};
use cosmic::{Apply, Element};
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
    /// Search
    Search,
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
            MenuAction::Search => Message::MenuAction(MenuAction::Search),
        }
    }
}

pub fn menu_bar<'a>(key_binds: &HashMap<KeyBind, MenuAction>) -> Element<'a, Message> {
    menu::bar(vec![
        menu::Tree::with_children(
            menu::root(fl!("file")).apply(Element::from),
            menu::items(
                key_binds,
                vec![
                    menu::Item::Button(fl!("new-vault-file"), None, MenuAction::NewVaultFile),
                    menu::Item::Button(fl!("new-folder"), None, MenuAction::NewVaultFolder),
                    menu::Item::Button(fl!("open-file"), None, MenuAction::OpenFile),
                    menu::Item::Button(fl!("save-file"), None, MenuAction::SaveFile),
                    menu::Item::Divider,
                    menu::Item::Button(fl!("new-file"), None, MenuAction::NewFile),
                ],
            ),
        ),
        menu::Tree::with_children(
            menu::root(fl!("edit")).apply(Element::from),
            menu::items(
                key_binds,
                vec![
                    menu::Item::Button(fl!("undo"), None, MenuAction::Undo),
                    menu::Item::Button(fl!("redo"), None, MenuAction::Redo),
                    menu::Item::Button(fl!("search"), None, MenuAction::Search),
                ],
            ),
        ),
        menu::Tree::with_children(
            menu::root(fl!("view")).apply(Element::from),
            menu::items(
                key_binds,
                vec![
                    menu::Item::Button(fl!("about"), None, MenuAction::About),
                    menu::Item::Button(fl!("settings"), None, MenuAction::Settings),
                ],
            ),
        ),
    ])
    .item_height(ItemHeight::Dynamic(40))
    .item_width(ItemWidth::Uniform(270))
    .spacing(4.0)
    .into()
}
