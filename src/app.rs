// SPDX-License-Identifier: GPL-3.0

use cosmic::iced::widget::{column, row};
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{container, text};
use cosmic::{Element, prelude::*, theme};

pub struct AppModel {
    core: cosmic::Core,
    cosmic_content: cosmic::iced::widget::text_editor::Content,
    custom_content: widgets::text_editor::Content,
}

/// Only two messages: one per editor.
#[derive(Debug, Clone)]
pub enum Message {
    /// Action fired by the built-in COSMIC/iced text editor.
    CosmicEdit(cosmic::iced::widget::text_editor::Action),
    /// Action fired by our custom text editor.
    CustomEdit(widgets::text_editor::Action),
}

impl cosmic::Application for AppModel {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = "dev.mariinkys.CedillaEditorDebug";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        let app = AppModel {
            core,
            cosmic_content: cosmic::iced::widget::text_editor::Content::new(),
            custom_content: widgets::text_editor::Content::new(),
        };

        (app, Task::none())
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::CosmicEdit(action) => {
                tracing::debug!(
                    target: "cedilla::editor::cosmic",
                    ?action,
                    "[COSMIC EDITOR] update"
                );
                self.cosmic_content.perform(action);
            }
            Message::CustomEdit(action) => {
                tracing::debug!(
                    target: "cedilla::editor::custom",
                    ?action,
                    "[CUSTOM EDITOR] update"
                );
                self.custom_content.perform(action);
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let spacing = theme::active().cosmic().spacing;

        let cosmic_pane = column![
            text::title4("Cosmic / iced built-in editor"),
            container(
                cosmic::iced::widget::text_editor(&self.cosmic_content)
                    .placeholder("Type here (cosmic editor)...")
                    .on_action(Message::CosmicEdit)
                    .height(Length::Fill)
            )
            .padding(spacing.space_xs)
            .width(Length::Fill)
            .height(Length::Fill)
            .class(theme::Container::Card),
        ]
        .spacing(spacing.space_xxs)
        .width(Length::FillPortion(1))
        .height(Length::Fill);

        let custom_pane = column![
            text::title4("Custom editor"),
            container(
                widgets::TextEditor::new(&self.custom_content)
                    .on_action(Message::CustomEdit)
                    .height(Length::Fill)
            )
            .padding(spacing.space_xs)
            .width(Length::Fill)
            .height(Length::Fill)
            .class(theme::Container::Card),
        ]
        .spacing(spacing.space_xxs)
        .width(Length::FillPortion(1))
        .height(Length::Fill);

        container(
            row![cosmic_pane, custom_pane]
                .spacing(spacing.space_s)
                .align_y(Alignment::Start),
        )
        .padding(spacing.space_s)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
