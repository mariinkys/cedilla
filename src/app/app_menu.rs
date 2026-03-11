// SPDX-License-Identifier: GPL-3.0

use crate::{app::Message, fl};
use cosmic::widget::menu::Item as MenuItem;
use cosmic::{
    Core, Element,
    widget::{
        menu::{self, ItemHeight, ItemWidth, KeyBind},
        responsive_menu_bar,
    },
};
use std::{collections::HashMap, sync::LazyLock};

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

//
// Responsive Menu Bar implementation based on cosmic-edit implementation (04/02/2026)
// Relevant links:
// https://github.com/pop-os/cosmic-edit/blob/master/src/menu.rs
// https://github.com/pop-os/cosmic-edit/blob/master/src/main.rs
//

static MENU_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(|| cosmic::widget::Id::new("responsive-menu"));

pub fn menu_bar<'a>(core: &Core, key_binds: &HashMap<KeyBind, MenuAction>) -> Element<'a, Message> {
    responsive_menu_bar()
        .item_height(ItemHeight::Dynamic(40))
        .item_width(ItemWidth::Uniform(270))
        .spacing(4.0)
        .into_element(
            core,
            key_binds,
            MENU_ID.clone(),
            Message::Surface,
            vec![
                (
                    fl!("file"),
                    vec![
                        MenuItem::Button(fl!("new-vault-file"), None, MenuAction::NewVaultFile),
                        MenuItem::Button(fl!("new-folder"), None, MenuAction::NewVaultFolder),
                        MenuItem::Button(fl!("open-file"), None, MenuAction::OpenFile),
                        MenuItem::Button(fl!("save-file"), None, MenuAction::SaveFile),
                        MenuItem::Divider,
                        MenuItem::Button(fl!("new-file"), None, MenuAction::NewFile),
                    ],
                ),
                (
                    fl!("edit"),
                    vec![
                        MenuItem::Button(fl!("undo"), None, MenuAction::Undo),
                        MenuItem::Button(fl!("redo"), None, MenuAction::Redo),
                    ],
                ),
                (
                    fl!("view"),
                    vec![
                        MenuItem::Button(fl!("about"), None, MenuAction::About),
                        MenuItem::Button(fl!("settings"), None, MenuAction::Settings),
                    ],
                ),
            ],
        )
}
