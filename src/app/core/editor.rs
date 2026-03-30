use std::path::PathBuf;

use cosmic::widget::text_editor::{Cursor, Position};
use widgets::text_editor;

use crate::app::core::{
    history::HistoryState, preview::MarkdownPreview, utils::search::SearchMatch,
};

pub struct EditorState {
    /// Current if/any file path
    pub path: Option<PathBuf>,
    /// Text Editor Content
    pub content: text_editor::Content,
    /// Track if any changes have been made to the current file
    pub is_dirty: bool,
    /// Allows us to undo and redo
    pub history: HistoryState,
    /// Holds state about the scrollbars/scrolling of the editor
    pub scroll: EditorScrollState,
    /// Holds the state of the search faetures of the editor
    pub search: EditorSearchState,
    /// Helper field to not open the external changes dialog when we save a file, move, delete...
    pub ignore_next_external_change: bool,
}

/// Allows us to correctly follow the cursor with the scrollbar
#[derive(Default)]
pub struct EditorScrollState {
    pub last_editor_viewport: Option<cosmic::iced_widget::scrollable::Viewport>,
    pub last_preview_viewport: Option<cosmic::iced_widget::scrollable::Viewport>,
    pub pending_editor_scrolls: u32,
    pub pending_preview_scrolls: u32,
    // we need this becasue the viewport get's calculated before images load-in then there's a weird jump in the preview
    pub last_preview_content_height: f32,
}

#[derive(Default)]
pub struct EditorSearchState {
    /// Controls wether the search box is shown or hidden
    pub show_search_box: bool,
    /// State of the search field
    pub search_value: String,
    /// Wether to use regex or not for searching
    pub use_regex: bool,
    /// Matches found (if any)
    pub matches: Vec<SearchMatch>,
    /// Contains the current match index
    pub current_match_index: Option<usize>,
    /// Errors parsing regex
    pub regex_error: Option<String>,
}

impl EditorState {
    pub fn push_history(&mut self, cursor_before: (usize, usize)) {
        let current_text = self.content.text();
        let cursor_after = self.content.cursor().position;

        // reconstruct the previous text so we can diff against it
        let prev_text = super::history::apply_patch(
            &self.history.history_base,
            &self.history.history_patches[..self.history.history_index],
        );
        let patch = super::history::make_patch(&prev_text, &current_text);

        // discard any redo patches above current index
        self.history
            .history_patches
            .truncate(self.history.history_index);
        self.history
            .cursor_before
            .truncate(self.history.history_index);
        self.history
            .cursor_after
            .truncate(self.history.history_index);

        self.history.history_patches.push(patch);
        self.history.cursor_before.push(cursor_before);
        self.history
            .cursor_after
            .push((cursor_after.line, cursor_after.column));
        self.history.history_index = self.history.history_patches.len();

        // keep only the last 100 patches; rebase onto the new base
        if self.history.history_patches.len() > 100 {
            // advance the base by applying the oldest patch
            let new_base = super::history::apply_single(
                &self.history.history_base,
                &self.history.history_patches[0],
            );
            self.history.history_base = new_base;
            self.history.cursor_before.remove(0);
            self.history.cursor_after.remove(0);
            self.history.history_index = self.history.history_patches.len();
        }
    }

    pub fn undo(&mut self, preview: &mut MarkdownPreview) {
        if self.history.history_index > 0 {
            let (line, col) = self
                .history
                .cursor_before
                .get(self.history.history_index - 1)
                .copied()
                .unwrap_or((0, 0));

            self.history.history_index -= 1;
            let snapshot = super::history::apply_patch(
                &self.history.history_base,
                &self.history.history_patches[..self.history.history_index],
            );

            self.content.replace_text(&snapshot);
            preview.update_content(&snapshot);

            self.is_dirty =
                self.history.history_index != 0 || !self.history.history_base.trim().is_empty();

            self.restore_cursor(line, col);
        }
    }

    pub fn redo(&mut self, preview: &mut MarkdownPreview) {
        if self.history.history_index < self.history.history_patches.len() {
            let (line, col) = self
                .history
                .cursor_after
                .get(self.history.history_index)
                .copied()
                .unwrap_or((0, 0));

            self.history.history_index += 1;
            let snapshot = super::history::apply_patch(
                &self.history.history_base,
                &self.history.history_patches[..self.history.history_index],
            );

            self.content.replace_text(&snapshot);
            preview.update_content(&snapshot);

            self.restore_cursor(line, col);
        }
    }

    /// Moves the cursor to a [`SearchMatch`] and selects the matched text.
    pub fn navigate_to_match(&mut self, m: &SearchMatch) {
        self.content.move_to(m.into());
    }

    /// Returns true if it's a vault path with any modification or if it's a new file with any content
    pub fn needs_confirmation(&self) -> bool {
        (self.path.is_some() && self.history.history_index != 0)
            || (self.path.is_none() && !self.content.text().trim().is_empty())
    }

    pub fn handle_list_continuation(&mut self) {
        match crate::app::utils::markdown::get_list_continuation(&self.content) {
            Some(continuation) if continuation.is_empty() => {
                // empty list item, clear the prefix and break out
                self.content
                    .perform(text_editor::Action::Move(text_editor::Motion::Home));
                self.content
                    .perform(text_editor::Action::Select(text_editor::Motion::End));
                self.content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Delete));
            }
            Some(continuation) => {
                self.content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Enter));
                self.content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                        std::sync::Arc::new(continuation),
                    )));
            }
            None => {
                self.content
                    .perform(text_editor::Action::Edit(text_editor::Edit::Enter));
            }
        }
    }

    pub fn handle_list_indent(&mut self) {
        let cursor_line = self.content.cursor().position.line;
        let line = self
            .content
            .line(cursor_line)
            .map(|l| l.text)
            .unwrap_or_default();

        if crate::app::utils::markdown::is_list_line(&line) {
            self.content
                .perform(text_editor::Action::Move(text_editor::Motion::Home));
            self.content
                .perform(text_editor::Action::Edit(text_editor::Edit::Paste(
                    std::sync::Arc::new("  ".to_string()),
                )));
            self.content
                .perform(text_editor::Action::Move(text_editor::Motion::End));
        } else {
            self.content
                .perform(text_editor::Action::Edit(text_editor::Edit::Insert('\t')));
        }
    }

    fn restore_cursor(&mut self, line: usize, column: usize) {
        let line_count = self.content.line_count();
        let safe_line = line.min(line_count.saturating_sub(1));

        // clamp column to actual char count of that line
        let safe_col = self
            .content
            .line(safe_line)
            .map(|l| column.min(l.text.chars().count()))
            .unwrap_or(0);

        self.content.move_to(Cursor {
            position: Position {
                line: safe_line,
                column: safe_col,
            },
            selection: None,
        });
    }
}
