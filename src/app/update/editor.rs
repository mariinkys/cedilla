// SPDX-License-Identifier: GPL-3.0

use crate::app::core::editor::{EditorSearchState, EditorState};
use crate::app::core::utils::search::SearchAction;
use crate::app::core::utils::{self};
use crate::app::{AppModel, Message, State, editor_scrollable_id, search_input_id, text_editor_id};
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
            editor.push_history();
        }

        let snap_task = snap_task(editor, was_edit);

        utils::images::download_images(
            &mut preview.markstate,
            &mut preview.images_in_progress,
            &editor.path,
        )
        .chain(snap_task)
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

        match action {
            SearchAction::ToggleSearch => {
                editor.search.show_search_box = !editor.search.show_search_box;
                // clear state when closing
                if !editor.search.show_search_box {
                    editor.search = EditorSearchState::default();
                    widgets::text_editor::focus(text_editor_id())
                        .chain(scroll_to_cursor_task(editor))
                } else {
                    cosmic::widget::text_input::focus(search_input_id())
                }
            }

            SearchAction::UpdateSearchValue(new_value) => {
                editor.search.search_value = new_value;
                editor.search.compute_matches(&editor.content.text());

                if let Some(idx) = editor.search.current_match_index {
                    editor.navigate_to_match(&editor.search.matches[idx].clone());
                    scroll_to_cursor_task(editor)
                } else {
                    Task::none()
                }
            }

            SearchAction::ToggleRegex => {
                editor.search.use_regex = !editor.search.use_regex;
                editor.search.compute_matches(&editor.content.text());

                if let Some(idx) = editor.search.current_match_index {
                    editor.navigate_to_match(&editor.search.matches[idx].clone());
                    scroll_to_cursor_task(editor)
                } else {
                    Task::none()
                }
            }

            SearchAction::NextResult => {
                if let Some(m) = editor.search.next_match().cloned() {
                    editor.navigate_to_match(&m);
                    scroll_to_cursor_task(editor)
                } else {
                    Task::none()
                }
            }

            SearchAction::PrevResult => {
                if let Some(m) = editor.search.prev_match().cloned() {
                    editor.navigate_to_match(&m);
                    scroll_to_cursor_task(editor)
                } else {
                    Task::none()
                }
            }

            SearchAction::FocusSearchField => cosmic::widget::text_input::focus(search_input_id()),
        }
    }
}

fn snap_task(editor: &mut EditorState, was_edit: bool) -> Task<cosmic::Action<Message>> {
    if was_edit {
        let total_lines = editor.content.line_count();
        let cursor_line = editor.content.cursor().position.line;

        if cursor_line + 1 >= total_lines {
            if let Some(vp) = editor.scroll.last_editor_viewport {
                let content_height = vp.content_bounds().height;
                let viewport_height = vp.bounds().height;
                let real_line_height = content_height / total_lines.max(1) as f32;
                let cursor_y = cursor_line as f32 * real_line_height;

                if cursor_y + real_line_height * 3.0
                    > editor.scroll.last_editor_scroll_y + viewport_height
                {
                    let new_y = (cursor_y + real_line_height * 3.0 - viewport_height)
                        .max(editor.scroll.last_editor_scroll_y);
                    editor.scroll.last_editor_scroll_y = new_y;
                    cosmic::iced_widget::scrollable::scroll_to(
                        editor_scrollable_id(),
                        cosmic::iced_widget::scrollable::AbsoluteOffset {
                            x: Some(0.0),
                            y: Some(new_y),
                        },
                    )
                    .map(cosmic::action::app)
                } else {
                    Task::none()
                }
            } else {
                Task::none()
            }
        } else {
            Task::none()
        }
    } else {
        Task::none()
    }
}

fn scroll_to_cursor_task(editor: &EditorState) -> Task<cosmic::Action<Message>> {
    let Some(vp) = editor.scroll.last_editor_viewport else {
        return Task::none();
    };

    let total_lines = editor.content.line_count();
    let cursor_line = editor.content.cursor().position.line;
    let content_height = vp.content_bounds().height;
    let viewport_height = vp.bounds().height;
    let real_line_height = content_height / total_lines.max(1) as f32;
    let cursor_y = cursor_line as f32 * real_line_height;

    let new_y = (cursor_y - viewport_height / 2.0).max(0.0);

    cosmic::iced_widget::scrollable::scroll_to(
        editor_scrollable_id(),
        cosmic::iced_widget::scrollable::AbsoluteOffset {
            x: Some(0.0),
            y: Some(new_y),
        },
    )
    .map(cosmic::action::app)
}
