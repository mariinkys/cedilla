// SPDX-License-Identifier: GPL-3.0

use crate::app::core::utils::Image;
use crate::app::{
    AppModel, Message, PreviewState, State, editor_scrollable_id, preview_scrollable_id,
};
use crate::config::BoolState;
use cosmic::iced_widget::{pane_grid, scrollable};
use cosmic::prelude::*;
use cosmic::widget::{self};
use frostmark::UpdateMsg;

impl AppModel {
    pub fn handle_update_mark_state(
        &mut self,
        message: UpdateMsg,
    ) -> Task<cosmic::Action<Message>> {
        if let State::Ready { preview, .. } = &mut self.state {
            preview.markstate.update(message)
        }
        Task::none()
    }

    pub fn handle_image_downloaded(
        &mut self,
        res: Result<Image, anywho::Error>,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { preview, .. } = &mut self.state else {
            return Task::none();
        };

        match res {
            Ok(image) => {
                if image.is_svg {
                    preview.insert_svg(image.url, image.bytes);
                } else {
                    preview.insert_image(image.url, image.bytes);
                }
            }
            Err(err) => {
                eprintln!("Couldn't download image: {err}");
            }
        }

        Task::none()
    }

    pub fn handle_set_preview_state(
        &mut self,
        desired_state: PreviewState,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { preview_state, .. } = &mut self.state else {
            return Task::none();
        };

        *preview_state = desired_state;

        Task::none()
    }

    pub fn handle_pane_resized(
        &mut self,
        event: pane_grid::ResizeEvent,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { panes, .. } = &mut self.state else {
            return Task::none();
        };

        panes.resize(event.split, event.ratio);
        Task::none()
    }

    pub fn handle_pane_dragged(
        &mut self,
        event: pane_grid::DragEvent,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { panes, .. } = &mut self.state else {
            return Task::none();
        };

        if let pane_grid::DragEvent::Dropped { pane, target } = event {
            panes.drop(pane, target);
        }

        Task::none()
    }

    pub fn handle_scroll_changed(
        &mut self,
        source_id: widget::Id,
        viewport: scrollable::Viewport,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready { editor, .. } = &mut self.state else {
            return Task::none();
        };

        if source_id == editor_scrollable_id() {
            editor.last_editor_viewport = Some(viewport);
            editor.last_editor_scroll_y = viewport.absolute_offset().y;
        }

        if self.config.scrollbar_sync != BoolState::Yes {
            return Task::none();
        }

        if source_id != editor_scrollable_id() {
            return Task::none();
        }

        let offset = viewport.absolute_offset();

        scrollable::scroll_to(preview_scrollable_id(), offset.into()).map(cosmic::action::app)
    }
}
