// SPDX-License-Identifier: GPL-3.0

use crate::app::core::utils::{self};
use crate::app::{AppModel, Message, State, editor_scrollable_id};
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
        editor.content.perform(action);
        preview.update_content(editor.content.text().as_ref());

        if was_edit {
            editor.is_dirty = true;
            editor.push_history();
        }

        let snap_task = if was_edit {
            let total_lines = editor.content.line_count();
            let cursor_line = editor.content.cursor().position.line;

            if cursor_line + 1 >= total_lines {
                if let Some(vp) = editor.last_editor_viewport {
                    let content_height = vp.content_bounds().height;
                    let viewport_height = vp.bounds().height;
                    let real_line_height = content_height / total_lines.max(1) as f32;
                    let cursor_y = cursor_line as f32 * real_line_height;

                    if cursor_y + real_line_height * 3.0
                        > editor.last_editor_scroll_y + viewport_height
                    {
                        let new_y = (cursor_y + real_line_height * 3.0 - viewport_height)
                            .max(editor.last_editor_scroll_y);
                        editor.last_editor_scroll_y = new_y;
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
        };

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
}
