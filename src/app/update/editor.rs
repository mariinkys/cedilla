// SPDX-License-Identifier: GPL-3.0

use crate::app::core::editor::{EditorSearchState, EditorState};
use crate::app::core::utils::search::SearchAction;
use crate::app::core::utils::{self};
use crate::app::{
    AppModel, Message, State, editor_scrollable_id, preview_scrollable_id, search_input_id,
    text_editor_id,
};
use crate::config::BoolState;
use cosmic::iced_widget::scrollable::scroll_to;
use cosmic::prelude::*;
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

        preview.update_content(editor.content.text().as_ref());

        if was_edit {
            editor.is_dirty = true;
            editor.push_history((cursor_before.line, cursor_before.column));
        }

        let sync_preview = self.config.scrollbar_sync == BoolState::Yes;
        let cursor_task = if was_edit {
            ensure_cursor_visible(editor, sync_preview)
        } else {
            Task::none()
        };

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
    let line_height = content_height / total_lines as f32;
    let cursor_top = cursor_line as f32 * line_height;
    let cursor_bottom = cursor_top + line_height;
    let scroll_y = editor_vp.absolute_offset().y;
    let padding = line_height * 3.0;

    let new_editor_y = if cursor_top < scroll_y + padding {
        // cursor above visible area
        (cursor_top - padding).max(0.0)
    } else if cursor_bottom > scroll_y + viewport_height - padding {
        // cursor below visible area
        cursor_bottom + padding - viewport_height
    } else {
        // already visible, nothing to do
        return Task::none();
    };

    // scroll editor, marking it as programmatic so it isn't re-synced via on_scroll
    editor.scroll.pending_editor_scrolls += 1;
    let editor_task = scroll_to(editor_scrollable_id(), utils::scroll::abs(new_editor_y))
        .map(cosmic::action::app);

    // if sync is active, also scroll the preview proportionally
    if let Some(preview_vp) = editor.scroll.last_preview_viewport
        && sync_preview
    {
        let editor_scrollable = (content_height - viewport_height).max(0.0);
        let rel = if editor_scrollable > 0.0 {
            new_editor_y / editor_scrollable
        } else {
            0.0
        };
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
