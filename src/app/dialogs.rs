use std::collections::VecDeque;

use cosmic::{Element, Task, theme, widget};

use crate::{app::Message, fl};

/// Represents a [`DialogPage`] of the application
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DialogPage {
    /// Dialog for creating a new file on the vault
    NewVaultFile(String),
}

impl DialogPage {
    /// View of the [`DialogPage`]
    pub fn display(&self, dialog_state: &DialogState) -> Option<Element<'_, Message>> {
        let spacing = theme::active().cosmic().spacing;

        let dialog = match &self {
            DialogPage::NewVaultFile(file_name) => widget::dialog()
                .title(fl!("new-vault-file"))
                .primary_action(
                    widget::button::suggested(fl!("create"))
                        .on_press(Message::DialogAction(DialogAction::DialogComplete)),
                )
                .secondary_action(
                    widget::button::standard(fl!("cancel"))
                        .on_press(Message::DialogAction(DialogAction::DialogCancel)),
                )
                .control(
                    widget::column::with_children(vec![
                        widget::text::body(fl!("file-name")).into(),
                        widget::text_input("", file_name.as_str())
                            .id(dialog_state.dialog_text_input.clone())
                            .on_input(move |name| {
                                Message::DialogAction(DialogAction::DialogUpdate(
                                    DialogPage::NewVaultFile(name),
                                ))
                            })
                            .on_submit(|_x| Message::DialogAction(DialogAction::DialogComplete))
                            .into(),
                    ])
                    .spacing(spacing.space_xxs),
                ),
        };

        Some(dialog.into())
    }
}

/// Represents an Action related to a Dialog
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DialogAction {
    /// Asks to open the [`DialogPage`] for creating a new vault file
    OpenNewVaultFileDialog,
    /// Action after user confirms/ok's/accepts the action of a Dialog
    DialogComplete,
    /// Action after user cancels the action of a Dialog
    DialogCancel,
    /// Updates the value of the given [`DialogPage`]
    DialogUpdate(DialogPage),
}

impl DialogAction {
    /// Executes the [`DialogAction`]
    pub fn execute(
        self,
        dialog_pages: &mut VecDeque<DialogPage>,
        dialog_state: &DialogState,
    ) -> Task<cosmic::Action<Message>> {
        match self {
            DialogAction::OpenNewVaultFileDialog => {
                dialog_pages.push_back(DialogPage::NewVaultFile(String::new()));
                widget::text_input::focus(dialog_state.dialog_text_input.clone())
            }
            DialogAction::DialogComplete => {
                if let Some(dialog_page) = dialog_pages.pop_front() {
                    match dialog_page {
                        DialogPage::NewVaultFile(file_name) => {
                            if !file_name.is_empty() {
                                return Task::done(cosmic::action::app(Message::NewVaultFile(
                                    file_name,
                                )));
                            }
                        }
                    }
                }
                Task::none()
            }
            DialogAction::DialogCancel => {
                dialog_pages.pop_front();
                Task::none()
            }
            DialogAction::DialogUpdate(dialog_page) => {
                dialog_pages[0] = dialog_page;
                Task::none()
            }
        }
    }
}

/// State of all the dialog widgets of the app
pub struct DialogState {
    /// Input inside of the Dialog Pages of the Application
    pub dialog_text_input: widget::Id,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            dialog_text_input: widget::Id::unique(),
        }
    }
}
