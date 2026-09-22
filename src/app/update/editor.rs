use crate::app::core::editor::{EditorSearchState, EditorState};
use crate::app::core::utils::search::SearchAction;
use crate::app::core::utils::{self};
use crate::app::core::vim::VimMode;
use crate::app::{
    AppModel, Message, State, VimAction, editor_scrollable_id, preview_scrollable_id, search_input_id,
    text_editor_id,
};
use crate::config::{BoolState, ConfigInput};
use cosmic::iced::widget::scrollable::scroll_to;
use cosmic::prelude::*;
use cosmic::widget::text_editor::{Cursor, Position};
use widgets::text_editor;

impl AppModel {
    pub fn handle_edit(&mut self, action: text_editor::Action) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor, preview, ..
        } = &mut self.state
        else {
            return Task::none();
        };

        let was_edit = action.is_edit();
        let cursor_before = editor.content.cursor().position;

        if let text_editor::Action::Edit(text_editor::Edit::Enter) = &action {
            editor.handle_list_continuation();
        } else if let text_editor::Action::Edit(text_editor::Edit::Insert('\t')) = &action {
            editor.handle_list_indent();
        } else {
            editor.content.perform(action);
        }

        if was_edit {
            preview.update_content(editor.content.text().as_ref());
            editor.is_dirty = true;
            editor.push_history((cursor_before.line, cursor_before.column));
        }

        let sync_preview = self.config.scrollbar_sync == BoolState::Yes;
        let cursor_task = ensure_cursor_visible(editor, sync_preview);

        if was_edit {
            utils::images::download_images(
                &mut preview.markstate,
                &mut preview.images_in_progress,
                &editor.path,
            )
            .chain(cursor_task)
        } else {
            cursor_task
        }
    }

    pub fn handle_apply_formatting(
        &mut self,
        action: utils::SelectionAction,
    ) -> Task<cosmic::Action<Message>> {
        self.apply_formatting_to_selection(action)
    }

    pub fn handle_paste_image(&mut self) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor, preview, ..
        } = &mut self.state
        else {
            return Task::none();
        };

        let target_dir = match &editor.path {
            Some(path) => path
                .parent()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| self.config.vault_path()),
            None => self.config.vault_path(),
        };

        match utils::images::save_clipboard_image(&target_dir) {
            Ok(file_name) => {
                let cursor_before = editor.content.cursor().position;
                let selection = editor.content.selection().unwrap_or_default();
                let alt = if selection.is_empty() { "" } else { &selection };
                let image_tag = format!("![{alt}]({file_name})");

                editor
                    .content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        std::sync::Arc::new(image_tag),
                    )));

                editor.is_dirty = true;
                editor.push_history((cursor_before.line, cursor_before.column));

                preview.update_content(editor.content.text().as_ref());

                let sync_preview = self.config.scrollbar_sync == BoolState::Yes;
                let cursor_task = ensure_cursor_visible(editor, sync_preview);

                utils::images::download_images(
                    &mut preview.markstate,
                    &mut preview.images_in_progress,
                    &editor.path,
                )
                .chain(cursor_task)
            }
            Err(err) => {
                eprintln!("Failed to paste image: {err}");
                self.handle_add_toast(utils::CedillaToast::new(crate::fl!(
                    "no-image-clipboard"
                )))
            }
        }
    }

    pub fn handle_smart_paste(&mut self) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor, preview, ..
        } = &mut self.state
        else {
            return Task::none();
        };

        let target_dir = match &editor.path {
            Some(path) => path
                .parent()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| self.config.vault_path()),
            None => self.config.vault_path(),
        };

        // Try pasting as an image first
        if let Ok(file_name) = utils::images::save_clipboard_image(&target_dir) {
            let cursor_before = editor.content.cursor().position;
            let selection = editor.content.selection().unwrap_or_default();
            let alt = if selection.is_empty() { "" } else { &selection };
            let image_tag = format!("![{alt}]({file_name})");

            editor
                .content
                .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                    std::sync::Arc::new(image_tag),
                )));

            editor.is_dirty = true;
            editor.push_history((cursor_before.line, cursor_before.column));

            preview.update_content(editor.content.text().as_ref());

            let sync_preview = self.config.scrollbar_sync == BoolState::Yes;
            let cursor_task = ensure_cursor_visible(editor, sync_preview);

            return utils::images::download_images(
                &mut preview.markstate,
                &mut preview.images_in_progress,
                &editor.path,
            )
            .chain(cursor_task);
        }

        // Fall back to standard text paste
        #[allow(clippy::collapsible_if)]
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                let cursor_before = editor.content.cursor().position;
                editor
                    .content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        std::sync::Arc::new(text),
                    )));

                editor.is_dirty = true;
                editor.push_history((cursor_before.line, cursor_before.column));

                preview.update_content(editor.content.text().as_ref());

                let sync_preview = self.config.scrollbar_sync == BoolState::Yes;
                return ensure_cursor_visible(editor, sync_preview);
            }
        }

        Task::none()
    }

    pub fn handle_undo(&mut self) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor, preview, ..
        } = &mut self.state
        else {
            return Task::none();
        };

        editor.undo(preview);

        utils::images::download_images(
            &mut preview.markstate,
            &mut preview.images_in_progress,
            &editor.path,
        )
    }

    pub fn handle_redo(&mut self) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor, preview, ..
        } = &mut self.state
        else {
            return Task::none();
        };

        editor.redo(preview);

        utils::images::download_images(
            &mut preview.markstate,
            &mut preview.images_in_progress,
            &editor.path,
        )
    }

    pub fn handle_search(&mut self, action: SearchAction) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        let sync_preview = self.config.scrollbar_sync == BoolState::Yes;

        match action {
            SearchAction::ToggleSearch => {
                editor.search.show_search_box = !editor.search.show_search_box;
                // clear state when closing
                if !editor.search.show_search_box {
                    editor.search = EditorSearchState::default();
                    widgets::text_editor::focus(text_editor_id())
                        .chain(ensure_cursor_visible(editor, sync_preview))
                } else {
                    cosmic::widget::text_input::focus(search_input_id())
                }
            }

            SearchAction::UpdateSearchValue(new_value) => {
                editor.search.search_value = new_value;
                editor.search.compute_matches(&editor.content.text());

                if let Some(idx) = editor.search.current_match_index {
                    editor.navigate_to_match(&editor.search.matches[idx].clone());
                    ensure_cursor_visible(editor, sync_preview)
                } else {
                    Task::none()
                }
            }

            SearchAction::ToggleRegex => {
                editor.search.use_regex = !editor.search.use_regex;
                editor.search.compute_matches(&editor.content.text());

                if let Some(idx) = editor.search.current_match_index {
                    editor.navigate_to_match(&editor.search.matches[idx].clone());
                    ensure_cursor_visible(editor, sync_preview)
                } else {
                    Task::none()
                }
            }

            SearchAction::NextResult => {
                if let Some(m) = editor.search.next_match().cloned() {
                    editor.navigate_to_match(&m);
                    ensure_cursor_visible(editor, sync_preview)
                } else {
                    Task::none()
                }
            }

            SearchAction::PrevResult => {
                if let Some(m) = editor.search.prev_match().cloned() {
                    editor.navigate_to_match(&m);
                    ensure_cursor_visible(editor, sync_preview)
                } else {
                    Task::none()
                }
            }

            SearchAction::FocusSearchField => cosmic::widget::text_input::focus(search_input_id()),
        }
    }

    pub fn handle_toggle_vim_mode(&mut self) -> Task<cosmic::Action<Message>> {
        let new_state = match self.config.vim_mode {
            BoolState::Yes => BoolState::No,
            BoolState::No => BoolState::Yes,
        };
        self.handle_config_input(ConfigInput::VimMode(new_state))
    }

    pub fn handle_vim_action(&mut self, action: VimAction) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor, preview, ..
        } = &mut self.state
        else {
            return Task::none();
        };

        let sync_preview = self.config.scrollbar_sync == BoolState::Yes;

        match action {
            VimAction::SetMode(mode) => {
                editor.vim.mode = mode;
                editor.vim.reset_operator_and_count();
                if mode == VimMode::Normal {
                    let pos = editor.content.cursor().position;
                    editor.content.move_to(Cursor {
                        position: pos,
                        selection: None,
                    });
                }
                Task::none()
            }
            VimAction::SetPendingOperator(op) => {
                editor.vim.pending_operator = op;
                Task::none()
            }
            VimAction::SetCountPrefix(count) => {
                editor.vim.count_prefix = count;
                Task::none()
            }
            VimAction::ResetOperatorAndCount => {
                editor.vim.reset_operator_and_count();
                Task::none()
            }
            VimAction::VisualYank { is_line } => {
                editor.vim.mode = VimMode::Normal;
                editor.vim.last_yank_is_line = is_line;
                editor.vim.reset_operator_and_count();
                let pos = editor.content.cursor().position;
                editor.content.move_to(Cursor {
                    position: pos,
                    selection: None,
                });
                Task::none()
            }
            VimAction::Escape => {
                editor.vim.mode = VimMode::Normal;
                editor.vim.reset_operator_and_count();
                let pos = editor.content.cursor().position;
                editor.content.move_to(Cursor {
                    position: pos,
                    selection: None,
                });
                Task::none()
            }
            VimAction::GoToTop => {
                editor.vim.reset_operator_and_count();
                editor.content.move_to(Cursor {
                    position: Position { line: 0, column: 0 },
                    selection: None,
                });
                ensure_cursor_visible(editor, sync_preview)
            }
            VimAction::GoToBottom => {
                editor.vim.reset_operator_and_count();
                let last_line = editor.content.line_count().saturating_sub(1);
                editor.content.move_to(Cursor {
                    position: Position {
                        line: last_line,
                        column: 0,
                    },
                    selection: None,
                });
                ensure_cursor_visible(editor, sync_preview)
            }
            VimAction::VisualGoToTop => {
                editor.vim.reset_operator_and_count();
                let anchor = editor
                    .content
                    .cursor()
                    .selection
                    .unwrap_or(editor.content.cursor().position);
                editor.content.move_to(Cursor {
                    position: Position { line: 0, column: 0 },
                    selection: Some(anchor),
                });
                ensure_cursor_visible(editor, sync_preview)
            }
            VimAction::VisualGoToBottom => {
                editor.vim.reset_operator_and_count();
                let anchor = editor
                    .content
                    .cursor()
                    .selection
                    .unwrap_or(editor.content.cursor().position);
                let last_line = editor.content.line_count().saturating_sub(1);
                let last_col = editor
                    .content
                    .line(last_line)
                    .map(|l| l.text.chars().count())
                    .unwrap_or(0);
                editor.content.move_to(Cursor {
                    position: Position {
                        line: last_line,
                        column: last_col,
                    },
                    selection: Some(anchor),
                });
                ensure_cursor_visible(editor, sync_preview)
            }
            VimAction::YankLine => {
                editor.vim.reset_operator_and_count();
                let line_idx = editor.content.cursor().position.line;
                if let Some(line) = editor.content.line(line_idx) {
                    let text_to_copy = format!("{}\n", line.text);
                    editor.vim.last_yank_is_line = true;
                    editor.vim.mode = VimMode::Normal;
                    self.handle_copy_to_clipboard(text_to_copy)
                } else {
                    Task::none()
                }
            }
            VimAction::DeleteLine => {
                editor.vim.reset_operator_and_count();
                let cursor_line = editor.content.cursor().position.line;
                let total_lines = editor.content.line_count();

                if let Some(line) = editor.content.line(cursor_line) {
                    let line_text = format!("{}\n", line.text);
                    editor.vim.last_yank_is_line = true;
                    if let Ok(mut clipboard) = arboard::Clipboard::new() {
                        let _ = clipboard.set_text(line_text);
                    }
                }

                let cursor_before = editor.content.cursor().position;

                if total_lines <= 1 {
                    editor.content.perform(text_editor::Action::SelectAll);
                    editor.content.perform(text_editor::Action::Edit(text_editor::Edit::Delete));
                } else if cursor_line + 1 < total_lines {
                    editor.content.move_to(Cursor {
                        position: Position {
                            line: cursor_line,
                            column: 0,
                        },
                        selection: None,
                    });
                    editor.content.move_to(Cursor {
                        position: Position {
                            line: cursor_line + 1,
                            column: 0,
                        },
                        selection: Some(Position {
                            line: cursor_line,
                            column: 0,
                        }),
                    });
                    editor.content.perform(text_editor::Action::Edit(text_editor::Edit::Delete));
                } else {
                    let prev_line = cursor_line - 1;
                    let prev_len = editor
                        .content
                        .line(prev_line)
                        .map(|l| l.text.chars().count())
                        .unwrap_or(0);
                    let cur_len = editor
                        .content
                        .line(cursor_line)
                        .map(|l| l.text.chars().count())
                        .unwrap_or(0);
                    editor.content.move_to(Cursor {
                        position: Position {
                            line: cursor_line,
                            column: cur_len,
                        },
                        selection: Some(Position {
                            line: prev_line,
                            column: prev_len,
                        }),
                    });
                    editor.content.perform(text_editor::Action::Edit(text_editor::Edit::Delete));
                }

                editor.is_dirty = true;
                editor.push_history((cursor_before.line, cursor_before.column));
                preview.update_content(editor.content.text().as_ref());
                ensure_cursor_visible(editor, sync_preview)
            }
            VimAction::Paste { before } => {
                editor.vim.reset_operator_and_count();
                if let Ok(mut clipboard) = arboard::Clipboard::new() {
                    if let Ok(text) = clipboard.get_text() {
                        if text.is_empty() {
                            return Task::none();
                        }
                        let is_line = text.ends_with('\n');
                        let cursor_before = editor.content.cursor().position;

                        if is_line {
                            if before {
                                editor
                                    .content
                                    .perform(text_editor::Action::Move(text_editor::Motion::Home));
                                editor
                                    .content
                                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                                        std::sync::Arc::new(text),
                                    )));
                            } else {
                                editor
                                    .content
                                    .perform(text_editor::Action::Move(text_editor::Motion::End));
                                editor
                                    .content
                                    .perform(text_editor::Action::Edit(text_editor::Edit::Enter));
                                editor
                                    .content
                                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                                        std::sync::Arc::new(
                                            text.trim_end_matches('\n').to_string(),
                                        ),
                                    )));
                            }
                        } else {
                            if !before {
                                editor.content.perform(text_editor::Action::Move(
                                    text_editor::Motion::Right,
                                ));
                            }
                            editor
                                .content
                                .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                                    std::sync::Arc::new(text),
                                )));
                        }

                        editor.is_dirty = true;
                        editor.push_history((cursor_before.line, cursor_before.column));
                        preview.update_content(editor.content.text().as_ref());
                        ensure_cursor_visible(editor, sync_preview)
                    } else {
                        Task::none()
                    }
                } else {
                    Task::none()
                }
            }
        }
    }
}

/// Scrolls the editor to keep the cursor visible.
fn ensure_cursor_visible(
    editor: &mut EditorState,
    sync_preview: bool,
) -> Task<cosmic::Action<Message>> {
    let Some(editor_vp) = editor.scroll.last_editor_viewport else {
        return Task::none();
    };

    let total_lines = editor.content.line_count().max(1);
    let cursor_line = editor.content.cursor().position.line;
    let content_height = editor_vp.content_bounds().height;
    let viewport_height = editor_vp.bounds().height;
    let max_scroll = (content_height - viewport_height).max(0.0);

    if max_scroll == 0.0 {
        return Task::none();
    }

    let line_height = content_height / total_lines as f32;
    let cursor_top = cursor_line as f32 * line_height;
    let cursor_bottom = cursor_top + line_height;
    let scroll_y = editor_vp.absolute_offset().y;
    let padding = (line_height * 3.0).min(viewport_height * 0.3);

    let new_editor_y = if cursor_top < scroll_y + padding {
        // cursor above visible area
        (cursor_top - padding).max(0.0)
    } else if cursor_bottom > scroll_y + viewport_height - padding {
        // cursor below visible area
        (cursor_bottom + padding - viewport_height).min(max_scroll)
    } else {
        // already visible, nothing to do
        return Task::none();
    };

    if (new_editor_y - scroll_y).abs() < 1.0 {
        return Task::none();
    }

    // scroll editor, marking it as programmatic so it isn't re-synced via on_scroll
    editor.scroll.pending_editor_scrolls += 1;
    let editor_task = scroll_to(editor_scrollable_id(), utils::scroll::abs(new_editor_y))
        .map(cosmic::action::app);

    // if sync is active, also scroll the preview proportionally
    if let Some(preview_vp) = editor.scroll.last_preview_viewport
        && sync_preview
    {
        let rel = new_editor_y / max_scroll;
        let preview_scrollable =
            (preview_vp.content_bounds().height - preview_vp.bounds().height).max(0.0);
        let new_preview_y = (rel * preview_scrollable).max(0.0);

        editor.scroll.pending_preview_scrolls += 1;
        let preview_task = scroll_to(preview_scrollable_id(), utils::scroll::abs(new_preview_y))
            .map(cosmic::action::app);

        return editor_task.chain(preview_task);
    }

    editor_task
}
