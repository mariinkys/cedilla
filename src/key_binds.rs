// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use cosmic::iced::keyboard::Key;
use cosmic::widget::menu::key_bind::KeyBind;
use cosmic::widget::menu::key_bind::Modifier;

use crate::app::app_menu::MenuAction;

/// COSMIC handled keybinds
pub fn key_binds() -> HashMap<KeyBind, MenuAction> {
    let mut key_binds = HashMap::new();

    macro_rules! bind {
        ([$($modifier:ident),* $(,)?], $key:expr, $action:ident) => {{
            key_binds.insert(
                KeyBind {
                    modifiers: vec![$(Modifier::$modifier),*],
                    key: $key,
                },
                MenuAction::$action,
            );
        }};
    }

    bind!([Ctrl], Key::Character("t".into()), NewVaultFile);
    bind!([Ctrl], Key::Character("n".into()), NewVaultFolder);
    bind!([Ctrl, Shift], Key::Character("t".into()), NewFile);
    bind!([Ctrl], Key::Character("s".into()), SaveFile);
    bind!([Ctrl], Key::Character("o".into()), OpenFile);

    bind!([Ctrl], Key::Character("z".into()), Undo);
    bind!([Ctrl, Shift], Key::Character("z".into()), Redo);

    bind!([Ctrl], Key::Character(",".into()), Settings);
    bind!([Ctrl], Key::Character("i".into()), About);

    bind!([Ctrl], Key::Character("h".into()), TogglePreview);

    key_binds
}
