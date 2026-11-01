// SPDX-License-Identifier: GPL-3.0

use crate::app::core::editor::{EditorSearchState, EditorState};
use crate::app::core::utils::clipboard::{ClipboardContent, PasteMode};
use crate::app::core::utils::search::SearchAction;
use crate::app::core::utils::{self};
use crate::app::{
    AppModel, Message, State, editor_scrollable_id, preview_scrollable_id, search_input_id,
    text_editor_id,
};
use crate::config::BoolState;
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

        // clicks, selections, cursor moves and scrolling don't change the text:
        // no need to re-render the preview or record history for them
        if !was_edit {
            return Task::none();
        }

        let text = editor.content.text();
        preview.update_content(&text);

        editor.is_dirty = true;
        editor.push_history_text((cursor_before.line, cursor_before.column), text);

        let sync_preview = self.config.scrollbar_sync == BoolState::Yes;
        let cursor_task = ensure_cursor_visible(editor, sync_preview);

        utils::images::download_images(
            &mut preview.markstate,
            &mut preview.images_in_progress,
            &editor.path,
        )
        .chain(cursor_task)
    }

    pub fn handle_apply_formatting(
        &mut self,
        action: utils::SelectionAction,
    ) -> Task<cosmic::Action<Message>> {
        self.apply_formatting_to_selection(action)
    }

    /// Ctrl+P / menu action: paste an image from the clipboard.
    pub fn handle_paste_image(&mut self) -> Task<cosmic::Action<Message>> {
        read_clipboard(PasteMode::ImageOnly)
    }

    /// Ctrl+V: pastes images as Markdown, falling back to plain text.
    pub fn handle_smart_paste(&mut self) -> Task<cosmic::Action<Message>> {
        read_clipboard(PasteMode::Smart)
    }

    pub fn handle_clipboard_data(
        &mut self,
        mode: PasteMode,
        content: Option<ClipboardContent>,
    ) -> Task<cosmic::Action<Message>> {
        match content {
            Some(content) => self.paste_clipboard_content(mode, content),
            // Nothing recognised was offered, or the backend can't read typed data (X11)
            None => cosmic::iced::clipboard::read()
                .map(move |text| cosmic::action::app(Message::ClipboardText(mode, text))),
        }
    }

    pub fn handle_clipboard_text(
        &mut self,
        mode: PasteMode,
        text: Option<String>,
    ) -> Task<cosmic::Action<Message>> {
        match text {
            Some(text) => self.paste_clipboard_content(mode, ClipboardContent::Text(text)),
            None => match mode {
                PasteMode::Smart => Task::none(),
                PasteMode::ImageOnly => self.no_image_in_clipboard_toast(),
            },
        }
    }

    fn paste_clipboard_content(
        &mut self,
        mode: PasteMode,
        content: ClipboardContent,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &self.state else {
            return Task::none();
        };

        let target_dir = match &editor.path {
            Some(path) => path
                .parent()
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| self.config.vault_path()),
            None => self.config.vault_path(),
        };

        let image_ref = match &content {
            ClipboardContent::Image { bytes, extension } => {
                utils::images::save_image_bytes(&target_dir, bytes, extension).map(Some)
            }
            ClipboardContent::Text(text) => {
                utils::images::copy_image_from_text_path(&target_dir, text)
            }
        };

        match (image_ref, content, mode) {
            (Ok(Some(file_name)), _, _) => self.insert_image_tag(&file_name),
            (Ok(None), ClipboardContent::Text(text), PasteMode::Smart) => {
                self.insert_pasted_text(text)
            }
            (Err(err), ClipboardContent::Text(text), PasteMode::Smart) => {
                eprintln!("Failed to paste image: {err}");
                self.insert_pasted_text(text)
            }
            (Err(err), _, PasteMode::Smart) => {
                eprintln!("Failed to paste image: {err}");
                Task::none()
            }
            (Err(err), _, PasteMode::ImageOnly) => {
                eprintln!("Failed to paste image: {err}");
                self.no_image_in_clipboard_toast()
            }
            (Ok(None), _, _) => self.no_image_in_clipboard_toast(),
        }
    }

    fn insert_image_tag(&mut self, file_name: &str) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor, preview, ..
        } = &mut self.state
        else {
            return Task::none();
        };

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

    fn insert_pasted_text(&mut self, text: String) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor, preview, ..
        } = &mut self.state
        else {
            return Task::none();
        };

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
        ensure_cursor_visible(editor, sync_preview)
    }

    fn no_image_in_clipboard_toast(&mut self) -> Task<cosmic::Action<Message>> {
        self.handle_add_toast(utils::CedillaToast::new(crate::fl!("no-image-clipboard")))
    }

    pub fn handle_toggle_checkbox(
        &mut self,
        index: usize,
        checked: bool,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor, preview, ..
        } = &mut self.state
        else {
            return Task::none();
        };

        let text = editor.content.text();
        let Some((line, column)) = find_task_marker(&text, index) else {
            return Task::none();
        };

        let cursor_before = editor.content.cursor().position;

        // select the single character between the brackets and replace it
        editor.content.move_to(Cursor {
            position: Position {
                line,
                column: column + 1,
            },
            selection: Some(Position { line, column }),
        });
        editor
            .content
            .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                std::sync::Arc::new(if checked { "x" } else { " " }.to_string()),
            )));

        // put the cursor back where the user had it
        editor.content.move_to(Cursor {
            position: cursor_before,
            selection: None,
        });

        editor.is_dirty = true;
        editor.push_history((cursor_before.line, cursor_before.column));

        preview.update_content(editor.content.text().as_ref());

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
}

/// Scrolls the editor (and the preview, if synced) so the cursor stays visible.
fn ensure_cursor_visible(
    editor: &mut EditorState,
    sync_preview: bool,
) -> Task<cosmic::Action<Message>> {
    let Some(editor_vp) = editor.scroll.last_editor_viewport else {
        return Task::none();
    };

    let viewport_height = editor_vp.bounds().height;
    let max_editor_y = utils::scroll::max_y(editor_vp);
    let scroll_y = editor_vp.absolute_offset().y;

    // real caret position, so wrapped lines are accounted for
    let cursor_top = editor.content.cursor_y();

    // approximate line height (only used for the cursor height and the margin around it)
    let total_lines = editor.content.line_count().max(1);
    let line_height = (editor_vp.content_bounds().height / total_lines as f32).max(1.0);
    let cursor_bottom = cursor_top + line_height;

    // keep a few lines of margin, but never so much that both conditions can be true at once
    let padding = (line_height * 3.0).min(((viewport_height - line_height) / 2.0).max(0.0));

    let new_editor_y = if cursor_top < scroll_y + padding {
        // cursor above visible area
        cursor_top - padding
    } else if cursor_bottom > scroll_y + viewport_height - padding {
        // cursor below visible area
        cursor_bottom + padding - viewport_height
    } else {
        // already visible, nothing to do
        return Task::none();
    }
    // no upper clamp: this runs before the edit is laid out, so `max_editor_y` is still the old
    // one (e.g. Enter on the last line). Iced clamps to the new content height itself.
    .max(0.0);

    // nothing would move (e.g. cursor at the very end of the document)
    if (new_editor_y - scroll_y).abs() < 0.5 {
        return Task::none();
    }

    // remember where we're going so the notification isn't treated as a user scroll
    editor.scroll.expected_editor_y = Some(new_editor_y);
    let editor_task = scroll_to(editor_scrollable_id(), utils::scroll::abs(new_editor_y))
        .map(cosmic::action::app);

    // if sync is active, also scroll the preview proportionally
    if let Some(preview_vp) = editor.scroll.last_preview_viewport
        && sync_preview
    {
        let rel = if max_editor_y > 0.0 {
            (new_editor_y / max_editor_y).min(1.0)
        } else {
            0.0
        };
        let new_preview_y = rel * utils::scroll::max_y(preview_vp);

        editor.scroll.expected_preview_y = Some(new_preview_y);
        let preview_task = scroll_to(preview_scrollable_id(), utils::scroll::abs(new_preview_y))
            .map(cosmic::action::app);

        return Task::batch([editor_task, preview_task]);
    }

    editor_task
}

/// Finds the nth task list marker (`- [ ]`, `* [x]`, `1. [ ]`...) in the text,
/// skipping fenced code blocks. Returns the line and the column (in chars) of
/// the character between the brackets.
fn find_task_marker(text: &str, index: usize) -> Option<(usize, usize)> {
    let mut in_fence = false;
    let mut count = 0;

    for (line_number, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }

        if let Some(column) = task_marker_column(line) {
            if count == index {
                return Some((line_number, column));
            }
            count += 1;
        }
    }

    None
}

/// If the line is a task list item, returns the column (in chars) of the
/// character between the brackets.
fn task_marker_column(line: &str) -> Option<usize> {
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    // indentation and blockquote markers
    while i < chars.len() && matches!(chars[i], ' ' | '\t' | '>') {
        i += 1;
    }

    // bullet (-, *, +) or ordered marker (1. or 1))
    if i < chars.len() && matches!(chars[i], '-' | '*' | '+') {
        i += 1;
    } else {
        let start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        if i == start || i >= chars.len() || !matches!(chars[i], '.' | ')') {
            return None;
        }
        i += 1;
    }

    // at least one space after the marker
    let start = i;
    while i < chars.len() && matches!(chars[i], ' ' | '\t') {
        i += 1;
    }
    if i == start {
        return None;
    }

    // [ ], [x] or [X], followed by whitespace or the end of the line
    let is_task = i + 2 < chars.len()
        && chars[i] == '['
        && matches!(chars[i + 1], ' ' | 'x' | 'X')
        && chars[i + 2] == ']'
        && chars.get(i + 3).is_none_or(|c| c.is_whitespace());

    is_task.then_some(i + 1)
}

/// Reads the clipboard through the window's own clipboard.
fn read_clipboard(mode: PasteMode) -> Task<cosmic::Action<Message>> {
    cosmic::iced::clipboard::read_data::<ClipboardContent>()
        .map(move |content| cosmic::action::app(Message::ClipboardData(mode, content)))
}
