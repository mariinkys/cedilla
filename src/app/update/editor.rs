// SPDX-License-Identifier: GPL-3.0

use crate::app::core;
use crate::app::core::utils::{self};
use crate::app::{AppModel, Message, State};
use cosmic::prelude::*;
use frostmark::MarkState;
use widgets::text_editor;

impl AppModel {
    pub fn handle_edit(&mut self, action: text_editor::Action) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            path,
            editor_content,
            is_dirty,
            markstate,
            images_in_progress,
            history,
            ..
        } = &mut self.state
        else {
            return Task::none();
        };

        let was_edit = action.is_edit();
        editor_content.perform(action);
        *markstate = MarkState::with_html_and_markdown(editor_content.text().as_ref());

        if was_edit {
            *is_dirty = true;
            let current_text = editor_content.text();

            // reconstruct the previous text so we can diff against it
            let prev_text = core::history::apply_patch(
                &history.history_base,
                &history.history_patches[..history.history_index],
            );
            let patch = core::history::make_patch(&prev_text, &current_text);

            // discard any redo patches above current index
            history.history_patches.truncate(history.history_index);
            history.history_patches.push(patch);
            history.history_index = history.history_patches.len();

            // keep only the last 100 patches; rebase onto the new base
            if history.history_patches.len() > 100 {
                // advance the base by applying the oldest patch
                let new_base =
                    core::history::apply_single(&history.history_base, &history.history_patches[0]);
                history.history_base = new_base;
                history.history_patches.remove(0);
                history.history_index = history.history_patches.len();
            }
        }

        utils::images::download_images(markstate, images_in_progress, path)
    }

    pub fn handle_apply_formatting(
        &mut self,
        action: utils::SelectionAction,
    ) -> Task<cosmic::Action<Message>> {
        self.apply_formatting_to_selection(action)
    }

    pub fn handle_undo(&mut self) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            path,
            editor_content,
            markstate,
            images_in_progress,
            is_dirty,
            history,
            ..
        } = &mut self.state
        else {
            return Task::none();
        };

        if history.history_index > 0 {
            history.history_index -= 1;
            let snapshot = core::history::apply_patch(
                &history.history_base,
                &history.history_patches[..history.history_index],
            );
            *editor_content = text_editor::Content::with_text(&snapshot);
            *markstate = MarkState::with_html_and_markdown(&snapshot);
            *is_dirty = history.history_index != 0 || !history.history_base.trim().is_empty();
        }

        utils::images::download_images(markstate, images_in_progress, path)
    }

    pub fn handle_redo(&mut self) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            path,
            editor_content,
            markstate,
            images_in_progress,
            history,
            ..
        } = &mut self.state
        else {
            return Task::none();
        };

        if history.history_index < history.history_patches.len() {
            history.history_index += 1;
            let snapshot = core::history::apply_patch(
                &history.history_base,
                &history.history_patches[..history.history_index],
            );
            *editor_content = text_editor::Content::with_text(&snapshot);
            *markstate = MarkState::with_html_and_markdown(&snapshot);
        }

        utils::images::download_images(markstate, images_in_progress, path)
    }
}
