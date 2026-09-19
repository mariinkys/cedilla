// SPDX-License-Identifier: GPL-3.0

use crate::app::context_page::ContextPage;
use crate::app::core::utils::CedillaToast;
use crate::app::{AppModel, Message};
use crate::fl;
use cosmic::iced::core::keyboard::{Key, Modifiers};
use cosmic::widget::ToastId;
use cosmic::{iced, prelude::*, surface};

impl AppModel {
    pub fn handle_close_toast(&mut self, id: ToastId) -> Task<cosmic::Action<Message>> {
        self.toasts.remove(id);
        Task::none()
    }

    pub fn handle_add_toast(&mut self, toast: CedillaToast) -> Task<cosmic::Action<Message>> {
        self.toasts.push(toast.into()).map(cosmic::action::app)
    }

    pub fn handle_launch_url(&mut self, url: String) -> Task<cosmic::Action<Message>> {
        match open::that_detached(&url) {
            Ok(()) => Task::none(),
            Err(err) => {
                eprintln!("failed to open {url:?}: {err}");
                Task::none()
            }
        }
    }

    pub fn handle_copy_to_clipboard(&mut self, content: String) -> Task<cosmic::Action<Message>> {
        iced::clipboard::write(content).chain(self.handle_add_toast(CedillaToast::new(fl!("copied-to-clipboard"))))
    }

    pub fn handle_toggle_context_page(
        &mut self,
        page: ContextPage,
    ) -> Task<cosmic::Action<Message>> {
        page.toggle_context_page(self)
    }

    pub fn handle_surface(&mut self, a: surface::Action<Message>) -> Task<cosmic::Action<Message>> {
        cosmic::task::message(cosmic::Action::Surface(a))
    }

    pub fn handle_key(&mut self, modifiers: Modifiers, key: Key) -> Task<cosmic::Action<Message>> {
        for (key_bind, action) in self.key_binds.iter() {
            if key_bind.matches(modifiers, &key, None) {
                return self.handle_menu_action(*action);
            }
        }
        Task::none()
    }

    pub fn handle_modifiers(&mut self, modifiers: Modifiers) -> Task<cosmic::Action<Message>> {
        self.modifiers = modifiers;
        Task::none()
    }
}
