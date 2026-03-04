// SPDX-License-Identifier: GPL-3.0

use cosmic::{Task, app::context_drawer};

use crate::{
    app::{AppModel, Message},
    fl,
};

/// The context page to display in the context drawer.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
pub enum ContextPage {
    #[default]
    /// About [`ContextPage`] of the application
    About,
    /// Settings [`ContextPage`] of the application
    Settings,
}

impl ContextPage {
    /// Display the [`ContextPage`]
    pub fn display<'a>(
        &self,
        app_model: &'a AppModel,
    ) -> Option<context_drawer::ContextDrawer<'a, Message>> {
        Some(match &self {
            ContextPage::About => context_drawer::about(
                &app_model.about,
                |s| Message::LaunchUrl(s.to_string()),
                Message::ToggleContextPage(ContextPage::About),
            )
            .title(fl!("about")),
            ContextPage::Settings => context_drawer::context_drawer(
                app_model.settings(),
                Message::ToggleContextPage(ContextPage::Settings),
            )
            .title(fl!("settings")),
        })
    }

    /// Toggle the current context page
    pub fn toggle_context_page(&self, state: &mut AppModel) -> Task<cosmic::Action<Message>> {
        if &state.context_page == self {
            // Close the context drawer if the toggled context page is the same.
            state.core.window.show_context = !state.core.window.show_context;
        } else {
            // Open the context drawer to display the requested context page.
            state.context_page = *self;
            state.core.window.show_context = true;
        }
        Task::none()
    }
}
