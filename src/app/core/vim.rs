// SPDX-License-Identifier: GPL-3.0

use cosmic::iced::keyboard::{self, key::Named, Key, Modifiers};
use widgets::text_editor::{Binding, KeyPress, Motion, Status};

use crate::app::{Message, VimAction};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VimMode {
    #[default]
    Normal,
    Insert,
    Visual,
    VisualLine,
}

impl VimMode {
    pub fn label(&self) -> &'static str {
        match self {
            VimMode::Normal => "-- NORMAL --",
            VimMode::Insert => "-- INSERT --",
            VimMode::Visual => "-- VISUAL --",
            VimMode::VisualLine => "-- VISUAL LINE --",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimPendingOperator {
    G, // 'g' pressed, waiting for 'g' (gg)
    Y, // 'y' pressed in Normal mode, waiting for 'y' (yy)
    D, // 'd' pressed in Normal mode, waiting for 'd' (dd)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VimState {
    pub mode: VimMode,
    pub count_prefix: Option<usize>,
    pub pending_operator: Option<VimPendingOperator>,
    pub last_yank_is_line: bool,
}

impl VimState {
    pub fn reset_operator_and_count(&mut self) {
        self.pending_operator = None;
        self.count_prefix = None;
    }
}

/// Translates a key press event in Vim mode into a text editor [`Binding`].
///
/// This is a pure function that does not mutate state. Any required state transitions
/// are returned as messages (`Message::Vim(VimAction::...)`) to be processed by the update function.
pub fn handle_vim_key_press(
    vim: &VimState,
    key_press: &KeyPress,
) -> Option<Binding<Message>> {
    let KeyPress {
        key,
        modified_key,
        physical_key,
        modifiers,
        status,
        ..
    } = key_press;

    if !matches!(status, Status::Focused { .. }) {
        return None;
    }

    // Always intercept Escape in Vim mode so the editor remains focused and returns to Normal mode.
    if matches!(modified_key, Key::Named(Named::Escape))
        || (modifiers.control() && matches!(key.to_latin(*physical_key), Some('[')))
    {
        let was_insert = vim.mode == VimMode::Insert;

        return if was_insert {
            Some(Binding::Sequence(vec![
                Binding::Move(Motion::Left),
                Binding::Custom(Message::Vim(VimAction::Escape)),
            ]))
        } else {
            Some(Binding::Custom(Message::Vim(VimAction::Escape)))
        };
    }

    match vim.mode {
        VimMode::Insert => {
            // In Insert mode, pass through all keys to standard text editor bindings
            None
        }
        VimMode::Normal => handle_normal_mode(vim, key, modified_key, *physical_key, *modifiers),
        VimMode::Visual => handle_visual_mode(vim, key, modified_key, *physical_key, *modifiers, false),
        VimMode::VisualLine => handle_visual_mode(vim, key, modified_key, *physical_key, *modifiers, true),
    }
}

fn handle_normal_mode(
    vim: &VimState,
    key: &Key,
    modified_key: &Key,
    physical_key: keyboard::key::Physical,
    modifiers: Modifiers,
) -> Option<Binding<Message>> {
    // Check pending operators first
    if let Some(pending) = vim.pending_operator {
        match pending {
            VimPendingOperator::G => {
                if matches!(key.to_latin(physical_key), Some('g')) {
                    return Some(Binding::Custom(Message::Vim(VimAction::GoToTop)));
                }
            }
            VimPendingOperator::Y => {
                if matches!(key.to_latin(physical_key), Some('y')) {
                    return Some(Binding::Custom(Message::Vim(VimAction::YankLine)));
                }
            }
            VimPendingOperator::D => {
                if matches!(key.to_latin(physical_key), Some('d')) {
                    return Some(Binding::Custom(Message::Vim(VimAction::DeleteLine)));
                }
            }
        }
    }

    // Ctrl shortcuts
    if modifiers.control() {
        match key.to_latin(physical_key) {
            Some('u') => return repeat_motion_lines(vim, Motion::Up, 15),
            Some('d') => return repeat_motion_lines(vim, Motion::Down, 15),
            Some('b') => return repeat_motion_lines(vim, Motion::Up, 30),
            Some('f') => return repeat_motion_lines(vim, Motion::Down, 30),
            Some('r') => {
                if vim.pending_operator.is_some() || vim.count_prefix.is_some() {
                    return Some(Binding::Sequence(vec![
                        Binding::Custom(Message::Redo),
                        Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
                    ]));
                } else {
                    return Some(Binding::Custom(Message::Redo));
                }
            }
            _ => return None,
        }
    }

    // Pass through system Command/Ctrl shortcuts (e.g. Save, Copy, Paste, etc.)
    if modifiers.command() {
        return None;
    }

    // Digits for count prefix (e.g. 5j, 10w)
    if !modifiers.control()
        && !modifiers.alt()
        && !modifiers.command()
        && let Key::Character(s) = key
        && let Some(c) = s.chars().next()
        && (('1'..='9').contains(&c) || (c == '0' && vim.count_prefix.is_some()))
    {
        let digit = c.to_digit(10).unwrap() as usize;
        let new_count = vim.count_prefix.unwrap_or(0) * 10 + digit;
        return Some(Binding::Custom(Message::Vim(VimAction::SetCountPrefix(
            Some(new_count),
        ))));
    }

    // Single-key commands and motions
    match modified_key {
        Key::Named(Named::ArrowLeft) => repeat_motion(vim, Motion::Left),
        Key::Named(Named::ArrowRight) => repeat_motion(vim, Motion::Right),
        Key::Named(Named::ArrowUp) => repeat_motion(vim, Motion::Up),
        Key::Named(Named::ArrowDown) => repeat_motion(vim, Motion::Down),
        Key::Named(Named::Home) => repeat_motion(vim, Motion::Home),
        Key::Named(Named::End) => repeat_motion(vim, Motion::End),
        Key::Named(Named::PageUp) => repeat_motion_lines(vim, Motion::Up, 30),
        Key::Named(Named::PageDown) => repeat_motion_lines(vim, Motion::Down, 30),
        Key::Named(Named::Enter) => repeat_motion(vim, Motion::Down),
        Key::Named(Named::Backspace) => repeat_motion(vim, Motion::Left),
        Key::Named(Named::Tab) => {
            if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
                Some(Binding::Custom(Message::Vim(
                    VimAction::ResetOperatorAndCount,
                )))
            } else {
                Some(Binding::Sequence(vec![]))
            }
        }
        _ => {
            if modifiers.control() || modifiers.alt() || modifiers.command() {
                return None;
            }

            match key.to_latin(physical_key) {
                // Movement: Directional
                Some(' ') => repeat_motion(vim, Motion::Right),
                Some('h') => repeat_motion(vim, Motion::Left),
                Some('j') => repeat_motion(vim, Motion::Down),
                Some('k') => repeat_motion(vim, Motion::Up),
                Some('l') => repeat_motion(vim, Motion::Right),

                // Movement: Words
                Some('w') => repeat_motion(vim, Motion::Right.widen()),
                Some('b') => repeat_motion(vim, Motion::Left.widen()),
                Some('e') => repeat_motion(vim, Motion::Right.widen()),

                // Movement: Line boundaries
                Some('0') | Some('^') => {
                    if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
                        Some(Binding::Sequence(vec![
                            Binding::Move(Motion::Home),
                            Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
                        ]))
                    } else {
                        Some(Binding::Move(Motion::Home))
                    }
                }
                Some('$') => {
                    if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
                        Some(Binding::Sequence(vec![
                            Binding::Move(Motion::End),
                            Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
                        ]))
                    } else {
                        Some(Binding::Move(Motion::End))
                    }
                }

                // Movement: Buffer boundaries
                Some('g') => Some(Binding::Custom(Message::Vim(
                    VimAction::SetPendingOperator(Some(VimPendingOperator::G)),
                ))),
                Some('G') if modifiers.shift() => {
                    Some(Binding::Custom(Message::Vim(VimAction::GoToBottom)))
                }

                // Mode Switching: Insert
                Some('i') => {
                    Some(Binding::Custom(Message::Vim(VimAction::SetMode(
                        VimMode::Insert,
                    ))))
                }
                Some('I') if modifiers.shift() => Some(Binding::Sequence(vec![
                    Binding::Move(Motion::Home),
                    Binding::Custom(Message::Vim(VimAction::SetMode(VimMode::Insert))),
                ])),
                Some('a') => Some(Binding::Sequence(vec![
                    Binding::Move(Motion::Right),
                    Binding::Custom(Message::Vim(VimAction::SetMode(VimMode::Insert))),
                ])),
                Some('A') if modifiers.shift() => Some(Binding::Sequence(vec![
                    Binding::Move(Motion::End),
                    Binding::Custom(Message::Vim(VimAction::SetMode(VimMode::Insert))),
                ])),
                Some('o') => Some(Binding::Sequence(vec![
                    Binding::Move(Motion::End),
                    Binding::Enter,
                    Binding::Custom(Message::Vim(VimAction::SetMode(VimMode::Insert))),
                ])),
                Some('O') if modifiers.shift() => Some(Binding::Sequence(vec![
                    Binding::Move(Motion::Home),
                    Binding::Enter,
                    Binding::Move(Motion::Up),
                    Binding::Custom(Message::Vim(VimAction::SetMode(VimMode::Insert))),
                ])),

                // Mode Switching: Visual
                Some('v') => {
                    Some(Binding::Custom(Message::Vim(VimAction::SetMode(
                        VimMode::Visual,
                    ))))
                }
                Some('V') if modifiers.shift() => Some(Binding::Sequence(vec![
                    Binding::SelectLine,
                    Binding::Custom(Message::Vim(VimAction::SetMode(VimMode::VisualLine))),
                ])),

                // Copy (Yank)
                Some('y') => Some(Binding::Custom(Message::Vim(
                    VimAction::SetPendingOperator(Some(VimPendingOperator::Y)),
                ))),
                Some('Y') if modifiers.shift() => {
                    Some(Binding::Custom(Message::Vim(VimAction::YankLine)))
                }

                // Delete (dd or D)
                Some('d') => Some(Binding::Custom(Message::Vim(
                    VimAction::SetPendingOperator(Some(VimPendingOperator::D)),
                ))),
                Some('D') if modifiers.shift() => Some(Binding::Sequence(vec![
                    Binding::Select(Motion::End),
                    Binding::Delete,
                    Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
                ])),

                // Paste (Put)
                Some('p') => {
                    Some(Binding::Custom(Message::Vim(VimAction::Paste { before: false })))
                }
                Some('P') if modifiers.shift() => {
                    Some(Binding::Custom(Message::Vim(VimAction::Paste { before: true })))
                }

                // Single character delete / Undo
                Some('x') => {
                    if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
                        Some(Binding::Sequence(vec![
                            Binding::Delete,
                            Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
                        ]))
                    } else {
                        Some(Binding::Delete)
                    }
                }
                Some('X') if modifiers.shift() => {
                    if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
                        Some(Binding::Sequence(vec![
                            Binding::Backspace,
                            Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
                        ]))
                    } else {
                        Some(Binding::Backspace)
                    }
                }
                Some('u') => {
                    if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
                        Some(Binding::Sequence(vec![
                            Binding::Custom(Message::Undo),
                            Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
                        ]))
                    } else {
                        Some(Binding::Custom(Message::Undo))
                    }
                }

                // Consume any other character key so NO letters are inserted in Normal mode
                _ => {
                    if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
                        Some(Binding::Custom(Message::Vim(
                            VimAction::ResetOperatorAndCount,
                        )))
                    } else {
                        Some(Binding::Sequence(vec![]))
                    }
                }
            }
        }
    }
}

fn handle_visual_mode(
    vim: &VimState,
    key: &Key,
    modified_key: &Key,
    physical_key: keyboard::key::Physical,
    modifiers: Modifiers,
    is_line: bool,
) -> Option<Binding<Message>> {
    // Ctrl shortcuts
    if modifiers.control() {
        match key.to_latin(physical_key) {
            Some('u') => return repeat_select_lines(vim, Motion::Up, 15),
            Some('d') => return repeat_select_lines(vim, Motion::Down, 15),
            Some('b') => return repeat_select_lines(vim, Motion::Up, 30),
            Some('f') => return repeat_select_lines(vim, Motion::Down, 30),
            _ => return None,
        }
    }

    if modifiers.command() {
        return None;
    }

    // Digits for count prefix
    if !modifiers.control()
        && !modifiers.alt()
        && !modifiers.command()
        && let Key::Character(s) = key
        && let Some(c) = s.chars().next()
        && (('1'..='9').contains(&c) || (c == '0' && vim.count_prefix.is_some()))
    {
        let digit = c.to_digit(10).unwrap() as usize;
        let new_count = vim.count_prefix.unwrap_or(0) * 10 + digit;
        return Some(Binding::Custom(Message::Vim(VimAction::SetCountPrefix(
            Some(new_count),
        ))));
    }

    // Single key motions / selections
    match modified_key {
        Key::Named(Named::ArrowLeft) => repeat_select(vim, Motion::Left),
        Key::Named(Named::ArrowRight) => repeat_select(vim, Motion::Right),
        Key::Named(Named::ArrowUp) => repeat_select(vim, Motion::Up),
        Key::Named(Named::ArrowDown) => repeat_select(vim, Motion::Down),
        Key::Named(Named::Home) => repeat_select(vim, Motion::Home),
        Key::Named(Named::End) => repeat_select(vim, Motion::End),
        Key::Named(Named::PageUp) => repeat_select_lines(vim, Motion::Up, 30),
        Key::Named(Named::PageDown) => repeat_select_lines(vim, Motion::Down, 30),
        Key::Named(Named::Enter) => repeat_select(vim, Motion::Down),
        Key::Named(Named::Backspace) => repeat_select(vim, Motion::Left),
        Key::Named(Named::Tab) => {
            if vim.count_prefix.is_some() {
                Some(Binding::Custom(Message::Vim(
                    VimAction::ResetOperatorAndCount,
                )))
            } else {
                Some(Binding::Sequence(vec![]))
            }
        }
        _ => {
            if modifiers.control() || modifiers.alt() || modifiers.command() {
                return None;
            }

            match key.to_latin(physical_key) {
                // Movement: Directional
                Some(' ') => repeat_select(vim, Motion::Right),
                Some('h') => repeat_select(vim, Motion::Left),
                Some('j') => repeat_select(vim, Motion::Down),
                Some('k') => repeat_select(vim, Motion::Up),
                Some('l') => repeat_select(vim, Motion::Right),

                // Movement: Words
                Some('w') => repeat_select(vim, Motion::Right.widen()),
                Some('b') => repeat_select(vim, Motion::Left.widen()),
                Some('e') => repeat_select(vim, Motion::Right.widen()),

                // Movement: Line boundaries
                Some('0') | Some('^') => {
                    if vim.count_prefix.is_some() {
                        Some(Binding::Sequence(vec![
                            Binding::Select(Motion::Home),
                            Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
                        ]))
                    } else {
                        Some(Binding::Select(Motion::Home))
                    }
                }
                Some('$') => {
                    if vim.count_prefix.is_some() {
                        Some(Binding::Sequence(vec![
                            Binding::Select(Motion::End),
                            Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
                        ]))
                    } else {
                        Some(Binding::Select(Motion::End))
                    }
                }

                // Movement: Buffer boundaries
                Some('g') => Some(Binding::Custom(Message::Vim(VimAction::VisualGoToTop))),
                Some('G') if modifiers.shift() => {
                    Some(Binding::Custom(Message::Vim(VimAction::VisualGoToBottom)))
                }

                // Copy (Yank) selected text
                Some('y') | Some('Y') => Some(Binding::Sequence(vec![
                    Binding::Copy,
                    Binding::Custom(Message::Vim(VimAction::VisualYank { is_line })),
                ])),

                // Delete selected text
                Some('d') | Some('D') | Some('x') | Some('X') => Some(Binding::Sequence(vec![
                    Binding::Delete,
                    Binding::Custom(Message::Vim(VimAction::SetMode(VimMode::Normal))),
                ])),

                // Paste (Replace selection with clipboard text)
                Some('p') | Some('P') => Some(Binding::Sequence(vec![
                    Binding::Paste,
                    Binding::Custom(Message::Vim(VimAction::SetMode(VimMode::Normal))),
                ])),

                // Toggle back to Normal mode on 'v' or 'V'
                Some('v') | Some('V') => {
                    Some(Binding::Custom(Message::Vim(VimAction::Escape)))
                }

                // Consume any other character key so NO letters are inserted in Visual mode
                _ => {
                    if vim.count_prefix.is_some() {
                        Some(Binding::Custom(Message::Vim(
                            VimAction::ResetOperatorAndCount,
                        )))
                    } else {
                        Some(Binding::Sequence(vec![]))
                    }
                }
            }
        }
    }
}

fn repeat_motion(vim: &VimState, motion: Motion) -> Option<Binding<Message>> {
    let count = vim.count_prefix.unwrap_or(1);
    if count <= 1 {
        if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
            Some(Binding::Sequence(vec![
                Binding::Move(motion),
                Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
            ]))
        } else {
            Some(Binding::Move(motion))
        }
    } else {
        let mut seq = vec![Binding::Move(motion); count];
        seq.push(Binding::Custom(Message::Vim(
            VimAction::ResetOperatorAndCount,
        )));
        Some(Binding::Sequence(seq))
    }
}

fn repeat_motion_lines(
    vim: &VimState,
    motion: Motion,
    default_lines: usize,
) -> Option<Binding<Message>> {
    let count = vim.count_prefix.unwrap_or(1);
    let total = count * default_lines;
    if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
        let mut seq = vec![Binding::Move(motion); total];
        seq.push(Binding::Custom(Message::Vim(
            VimAction::ResetOperatorAndCount,
        )));
        Some(Binding::Sequence(seq))
    } else {
        Some(Binding::Sequence(vec![Binding::Move(motion); total]))
    }
}

fn repeat_select(vim: &VimState, motion: Motion) -> Option<Binding<Message>> {
    let count = vim.count_prefix.unwrap_or(1);
    if count <= 1 {
        if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
            Some(Binding::Sequence(vec![
                Binding::Select(motion),
                Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount)),
            ]))
        } else {
            Some(Binding::Select(motion))
        }
    } else {
        let mut seq = vec![Binding::Select(motion); count];
        seq.push(Binding::Custom(Message::Vim(
            VimAction::ResetOperatorAndCount,
        )));
        Some(Binding::Sequence(seq))
    }
}

fn repeat_select_lines(
    vim: &VimState,
    motion: Motion,
    default_lines: usize,
) -> Option<Binding<Message>> {
    let count = vim.count_prefix.unwrap_or(1);
    let total = count * default_lines;
    if vim.count_prefix.is_some() || vim.pending_operator.is_some() {
        let mut seq = vec![Binding::Select(motion); total];
        seq.push(Binding::Custom(Message::Vim(
            VimAction::ResetOperatorAndCount,
        )));
        Some(Binding::Sequence(seq))
    } else {
        Some(Binding::Sequence(vec![Binding::Select(motion); total]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic::iced::keyboard::key::Named;

    fn make_key_press(key: Key, modifiers: Modifiers) -> KeyPress {
        KeyPress {
            key: key.clone(),
            modified_key: key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            modifiers,
            text: None,
            status: Status::Focused { is_hovered: true },
        }
    }

    #[test]
    fn test_normal_mode_motions() {
        let vim = VimState::default();
        let kp = make_key_press(Key::Character("j".into()), Modifiers::default());
        let res = handle_vim_key_press(&vim, &kp);
        assert!(matches!(res, Some(Binding::Move(Motion::Down))));
    }

    #[test]
    fn test_count_prefix_motion() {
        let vim = VimState::default();
        let kp5 = make_key_press(Key::Character("5".into()), Modifiers::default());
        let res1 = handle_vim_key_press(&vim, &kp5);
        assert!(matches!(
            res1,
            Some(Binding::Custom(Message::Vim(VimAction::SetCountPrefix(Some(5)))))
        ));

        let vim_with_count = VimState {
            count_prefix: Some(5),
            ..Default::default()
        };
        let kpj = make_key_press(Key::Character("j".into()), Modifiers::default());
        let res2 = handle_vim_key_press(&vim_with_count, &kpj);
        if let Some(Binding::Sequence(seq)) = res2 {
            assert_eq!(seq.len(), 6);
            assert!(matches!(seq[0], Binding::Move(Motion::Down)));
            assert!(matches!(
                seq[5],
                Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount))
            ));
        } else {
            panic!("Expected sequence of 5 moves + reset");
        }
    }

    #[test]
    fn test_mode_transitions() {
        let vim = VimState::default();
        let kpi = make_key_press(Key::Character("i".into()), Modifiers::default());
        let res = handle_vim_key_press(&vim, &kpi);
        assert!(matches!(
            res,
            Some(Binding::Custom(Message::Vim(VimAction::SetMode(
                VimMode::Insert
            ))))
        ));

        let vim_insert = VimState {
            mode: VimMode::Insert,
            ..Default::default()
        };
        let kpesc = make_key_press(Key::Named(Named::Escape), Modifiers::default());
        let res_esc = handle_vim_key_press(&vim_insert, &kpesc);
        assert!(matches!(res_esc, Some(Binding::Sequence(_))));
    }

    #[test]
    fn test_visual_mode_yank() {
        let vim = VimState {
            mode: VimMode::Visual,
            ..Default::default()
        };
        let kpy = make_key_press(Key::Character("y".into()), Modifiers::default());
        let res = handle_vim_key_press(&vim, &kpy);
        if let Some(Binding::Sequence(seq)) = res {
            assert_eq!(seq.len(), 2);
            assert!(matches!(seq[0], Binding::Copy));
            assert!(matches!(
                seq[1],
                Binding::Custom(Message::Vim(VimAction::VisualYank { is_line: false }))
            ));
        } else {
            panic!("Expected sequence of Copy + VisualYank");
        }
    }

    #[test]
    fn test_normal_mode_dd_deletes_line() {
        let vim = VimState::default();
        let kpd = make_key_press(Key::Character("d".into()), Modifiers::default());
        let res1 = handle_vim_key_press(&vim, &kpd);
        assert!(matches!(
            res1,
            Some(Binding::Custom(Message::Vim(
                VimAction::SetPendingOperator(Some(VimPendingOperator::D))
            )))
        ));

        let vim_pending = VimState {
            pending_operator: Some(VimPendingOperator::D),
            ..Default::default()
        };
        let res2 = handle_vim_key_press(&vim_pending, &kpd);
        assert!(matches!(
            res2,
            Some(Binding::Custom(Message::Vim(VimAction::DeleteLine)))
        ));
    }

    #[test]
    fn test_normal_mode_unhandled_letters_consumed() {
        let vim = VimState::default();
        let kpz = make_key_press(Key::Character("z".into()), Modifiers::default());
        let res = handle_vim_key_press(&vim, &kpz);
        // Unhandled letters must return an empty sequence to consume the key without inserting text
        assert!(matches!(res, Some(Binding::Sequence(seq)) if seq.is_empty()));
    }

    #[test]
    fn test_visual_mode_delete() {
        let vim = VimState {
            mode: VimMode::Visual,
            ..Default::default()
        };
        let kpd = make_key_press(Key::Character("d".into()), Modifiers::default());
        let res = handle_vim_key_press(&vim, &kpd);
        if let Some(Binding::Sequence(seq)) = res {
            assert_eq!(seq.len(), 2);
            assert!(matches!(seq[0], Binding::Delete));
            assert!(matches!(
                seq[1],
                Binding::Custom(Message::Vim(VimAction::SetMode(VimMode::Normal)))
            ));
        } else {
            panic!("Expected sequence of Delete + SetMode");
        }
    }

    #[test]
    fn test_ctrl_u_ctrl_d_page_motions() {
        let vim = VimState::default();
        let kpd = make_key_press(Key::Character("d".into()), Modifiers::CTRL);
        let res = handle_vim_key_press(&vim, &kpd);
        if let Some(Binding::Sequence(seq)) = res {
            assert_eq!(seq.len(), 15);
            assert!(matches!(seq[0], Binding::Move(Motion::Down)));
        } else {
            panic!("Expected sequence of 15 moves down");
        }

        let kpu = make_key_press(Key::Character("u".into()), Modifiers::CTRL);
        let res = handle_vim_key_press(&vim, &kpu);
        if let Some(Binding::Sequence(seq)) = res {
            assert_eq!(seq.len(), 15);
            assert!(matches!(seq[0], Binding::Move(Motion::Up)));
        } else {
            panic!("Expected sequence of 15 moves up");
        }
    }

    #[test]
    fn test_normal_mode_gg_goto_top() {
        let vim = VimState::default();
        let kpg = make_key_press(Key::Character("g".into()), Modifiers::default());
        let res1 = handle_vim_key_press(&vim, &kpg);
        assert!(matches!(
            res1,
            Some(Binding::Custom(Message::Vim(
                VimAction::SetPendingOperator(Some(VimPendingOperator::G))
            )))
        ));

        let vim_pending = VimState {
            pending_operator: Some(VimPendingOperator::G),
            ..Default::default()
        };
        let res2 = handle_vim_key_press(&vim_pending, &kpg);
        assert!(matches!(
            res2,
            Some(Binding::Custom(Message::Vim(VimAction::GoToTop)))
        ));
    }

    #[test]
    fn test_normal_mode_yy_yank_line() {
        let vim = VimState::default();
        let kpy = make_key_press(Key::Character("y".into()), Modifiers::default());
        let res1 = handle_vim_key_press(&vim, &kpy);
        assert!(matches!(
            res1,
            Some(Binding::Custom(Message::Vim(
                VimAction::SetPendingOperator(Some(VimPendingOperator::Y))
            )))
        ));

        let vim_pending = VimState {
            pending_operator: Some(VimPendingOperator::Y),
            ..Default::default()
        };
        let res2 = handle_vim_key_press(&vim_pending, &kpy);
        assert!(matches!(
            res2,
            Some(Binding::Custom(Message::Vim(VimAction::YankLine)))
        ));
    }

    #[test]
    fn test_normal_mode_zero_vs_count_prefix() {
        let vim = VimState::default();
        let kp0 = make_key_press(Key::Character("0".into()), Modifiers::default());
        let res0 = handle_vim_key_press(&vim, &kp0);
        // '0' alone moves to Home
        assert!(matches!(res0, Some(Binding::Move(Motion::Home))));

        // '1' followed by '0' sets count to 10
        let vim_with_1 = VimState {
            count_prefix: Some(1),
            ..Default::default()
        };
        let res10 = handle_vim_key_press(&vim_with_1, &kp0);
        assert!(matches!(
            res10,
            Some(Binding::Custom(Message::Vim(VimAction::SetCountPrefix(Some(10)))))
        ));
    }

    #[test]
    fn test_pending_operator_cancelled_by_motion() {
        let vim = VimState {
            pending_operator: Some(VimPendingOperator::D),
            ..Default::default()
        };
        let kpj = make_key_press(Key::Character("j".into()), Modifiers::default());
        let res = handle_vim_key_press(&vim, &kpj);
        if let Some(Binding::Sequence(seq)) = res {
            assert_eq!(seq.len(), 2);
            assert!(matches!(seq[0], Binding::Move(Motion::Down)));
            assert!(matches!(
                seq[1],
                Binding::Custom(Message::Vim(VimAction::ResetOperatorAndCount))
            ));
        } else {
            panic!("Expected sequence of Move + ResetOperatorAndCount");
        }
    }
}
