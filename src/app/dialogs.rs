use std::{collections::VecDeque, path::PathBuf};

use cosmic::{Element, Task, iced::Alignment, theme, widget};

use crate::{app::Message, fl};

/// Represents a [`DialogPage`] of the application
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DialogPage {
    /// Dialog for creating a new file on the vault
    NewVaultFile(String),
    /// Dialog for creating a new folder on the vault
    NewVaultFolder(String),
    /// Delete the currently selected folder/file
    DeleteNode(cosmic::widget::segmented_button::Entity),
    /// Rename the currently selected folder/file
    RenameNode(cosmic::widget::segmented_button::Entity, String),
    /// Move the given entity to a selected folder first entity = what to move, second = selected target folder (None if none chosen yet)
    MoveNode(cosmic::widget::segmented_button::Entity, Option<PathBuf>),
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
            DialogPage::NewVaultFolder(folder_name) => widget::dialog()
                .title(fl!("new-folder"))
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
                        widget::text::body(fl!("folder-name")).into(),
                        widget::text_input("", folder_name.as_str())
                            .id(dialog_state.dialog_text_input.clone())
                            .on_input(move |name| {
                                Message::DialogAction(DialogAction::DialogUpdate(
                                    DialogPage::NewVaultFolder(name),
                                ))
                            })
                            .on_submit(|_x| Message::DialogAction(DialogAction::DialogComplete))
                            .into(),
                    ])
                    .spacing(spacing.space_xxs),
                ),
            DialogPage::DeleteNode(_entity) => widget::dialog()
                .title(fl!("delete-node"))
                .primary_action(
                    widget::button::suggested(fl!("delete"))
                        .on_press(Message::DialogAction(DialogAction::DialogComplete)),
                )
                .secondary_action(
                    widget::button::standard(fl!("cancel"))
                        .on_press(Message::DialogAction(DialogAction::DialogCancel)),
                )
                .control(
                    widget::column::with_children(vec![
                        widget::text::body(fl!("delete-confirmation")).into(),
                    ])
                    .spacing(spacing.space_xxs),
                ),
            DialogPage::RenameNode(entity, new_name) => widget::dialog()
                .title(fl!("rename"))
                .primary_action(
                    widget::button::suggested(fl!("rename"))
                        .on_press(Message::DialogAction(DialogAction::DialogComplete)),
                )
                .secondary_action(
                    widget::button::standard(fl!("cancel"))
                        .on_press(Message::DialogAction(DialogAction::DialogCancel)),
                )
                .control(
                    widget::column::with_children(vec![
                        widget::text::body(fl!("name")).into(),
                        widget::text_input("", new_name.as_str())
                            .id(dialog_state.dialog_text_input.clone())
                            .on_input(move |name| {
                                Message::DialogAction(DialogAction::DialogUpdate(
                                    DialogPage::RenameNode(*entity, name),
                                ))
                            })
                            .on_submit(|_x| Message::DialogAction(DialogAction::DialogComplete))
                            .into(),
                    ])
                    .spacing(spacing.space_xxs),
                ),
            DialogPage::MoveNode(source_entity, selected_target) => {
                let folder_list = widget::column::with_children(
                    dialog_state
                        .available_folders
                        .iter()
                        .map(|(path, name, indent)| {
                            let is_selected = selected_target.as_ref() == Some(path);
                            let indent_padding = (*indent as f32) * 32.0;

                            widget::button::custom(
                                widget::row::with_children(vec![
                                    widget::Space::with_width(indent_padding).into(),
                                    widget::icon::from_name("folder-symbolic").size(16).into(),
                                    widget::text::body(name.clone()).into(),
                                ])
                                .align_y(Alignment::Center)
                                .spacing(spacing.space_xxs),
                            )
                            .on_press(Message::DialogAction(DialogAction::DialogUpdate(
                                DialogPage::MoveNode(*source_entity, Some(path.clone())),
                            )))
                            .class(if is_selected {
                                theme::Button::Suggested
                            } else {
                                theme::Button::MenuItem
                            })
                            .width(cosmic::iced::Length::Fill)
                            .into()
                        })
                        .collect::<Vec<Element<Message>>>(),
                )
                .spacing(spacing.space_xxxs);

                widget::dialog()
                    .title(fl!("move-to"))
                    .primary_action(
                        widget::button::suggested(fl!("move")).on_press_maybe(
                            selected_target
                                .as_ref()
                                .map(|_| Message::DialogAction(DialogAction::DialogComplete)),
                        ),
                    )
                    .secondary_action(
                        widget::button::standard(fl!("cancel"))
                            .on_press(Message::DialogAction(DialogAction::DialogCancel)),
                    )
                    .control(
                        widget::scrollable(folder_list).height(cosmic::iced::Length::Fixed(300.0)),
                    )
            }
        };

        Some(dialog.into())
    }
}

/// Represents an Action related to a Dialog
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DialogAction {
    /// Asks to open the [`DialogPage`] for creating a new vault file
    OpenNewVaultFileDialog,
    /// Asks to open the [`DialogPage`] for creating a new vault folder
    OpenNewVaultFolderDialog,
    /// Asks to open the [`DialogPage`] for deleting a vault folder/file
    OpenDeleteNodeDialog(cosmic::widget::segmented_button::Entity),
    /// Asks to open the [`DialogPage`] for renaming a vault folder/file
    OpenRenameNodeDialog(cosmic::widget::segmented_button::Entity),
    /// Asks to open the [`DialogPage`] for moving a node in the vault
    OpenMoveNodeDialog(cosmic::widget::segmented_button::Entity),
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
            DialogAction::OpenNewVaultFolderDialog => {
                dialog_pages.push_back(DialogPage::NewVaultFolder(String::new()));
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
                        DialogPage::NewVaultFolder(folder_name) => {
                            if !folder_name.is_empty() {
                                return Task::done(cosmic::action::app(Message::NewVaultFolder(
                                    folder_name,
                                )));
                            }
                        }
                        DialogPage::DeleteNode(entity) => {
                            return Task::done(cosmic::action::app(Message::DeleteNode(entity)));
                        }
                        DialogPage::RenameNode(entity, new_name) => {
                            if !new_name.is_empty() {
                                return Task::done(cosmic::action::app(Message::RenameNode(
                                    entity, new_name,
                                )));
                            }
                        }
                        DialogPage::MoveNode(source_entity, Some(target_path)) => {
                            return Task::done(cosmic::action::app(Message::MoveNode(
                                source_entity,
                                target_path,
                            )));
                        }
                        DialogPage::MoveNode(_, None) => {}
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
            DialogAction::OpenDeleteNodeDialog(entity) => {
                dialog_pages.push_back(DialogPage::DeleteNode(entity));
                Task::none()
            }
            DialogAction::OpenRenameNodeDialog(entity) => {
                dialog_pages.push_back(DialogPage::RenameNode(entity, String::new()));
                widget::text_input::focus(dialog_state.dialog_text_input.clone())
            }
            DialogAction::OpenMoveNodeDialog(entity) => {
                dialog_pages.push_back(DialogPage::MoveNode(entity, None));
                Task::none()
            }
        }
    }
}

/// State of all the dialog widgets of the app
pub struct DialogState {
    /// Input inside of the Dialog Pages of the Application
    pub dialog_text_input: widget::Id,
    /// Available folders to move a node in
    pub available_folders: Vec<(PathBuf, String, u16)>,
}

impl Default for DialogState {
    fn default() -> Self {
        Self {
            dialog_text_input: widget::Id::unique(),
            available_folders: Vec::new(),
        }
    }
}
