use std::sync::Arc;

use cosmic::Task;

use crate::app::{
    AppModel, Message, State,
    widgets::{markdown, text_editor},
};

/// Actions that can be performed on the current text selection
#[derive(Debug, Clone)]
pub enum SelectionAction {
    /// Convert selection to Heading 1 / Insert empty heading 1
    Heading1,
    /// Convert selection to Heading 2 / Insert empty heading 2
    Heading2,
    /// Convert selection to Heading 3 / Insert empty heading 3
    Heading3,
    /// Convert selection to Heading 4 / Insert empty heading 4
    Heading4,
    /// Convert selection to Heading 5 / Insert empty heading 5
    Heading5,
    /// Convert selection to Heading 6 / Insert empty heading 6
    Heading6,
    /// Convert selection to bold / Insert bold markers
    Bold,
    /// Convert selection to italic / Insert italic markers
    Italic,
    /// Convert selection to hyperlink / Insert hyperlink template
    Hyperlink,
    /// Convert selection to inline code / Insert code markers
    Code,
    /// Insert code block
    CodeBlock,
    /// Convert selection to image / Insert image template
    Image,
    /// Convert selection to bulleted list / Insert list item
    BulletedList,
    /// Convert selection to numbered list / Insert numbered item
    NumberedList,
    /// Convert selection to checkbox list / Insert checkbox item
    CheckboxList,
    /// Add horizontal rule
    Rule,
}

impl AppModel {
    /// Apply formatting to the currently selected text in the editor
    pub fn apply_formatting_to_selection(
        &mut self,
        action: SelectionAction,
    ) -> Task<cosmic::Action<Message>> {
        let State::Ready {
            editor_content,
            is_dirty,
            items,
            ..
        } = &mut self.state
        else {
            return Task::none();
        };

        // format the selected text
        let formatted =
            format_selected_text(&editor_content.selection().unwrap_or_default(), action);

        // replace the selection (or insert if no selection)
        editor_content.perform(text_editor::Action::Edit(text_editor::Edit::Paste(
            Arc::new(formatted),
        )));

        *is_dirty = true;
        *items = markdown::parse(editor_content.text().as_ref()).collect();

        Task::none()
    }
}

/// Format the selected text based on the action
fn format_selected_text(text: &str, action: SelectionAction) -> String {
    let is_empty = text.is_empty();

    match action {
        SelectionAction::Heading1 => format_heading(text, 1),
        SelectionAction::Heading2 => format_heading(text, 2),
        SelectionAction::Heading3 => format_heading(text, 3),
        SelectionAction::Heading4 => format_heading(text, 4),
        SelectionAction::Heading5 => format_heading(text, 5),
        SelectionAction::Heading6 => format_heading(text, 6),

        SelectionAction::Bold => {
            if is_empty {
                "**".to_string()
            } else {
                format!("**{}**", text)
            }
        }

        SelectionAction::Italic => {
            if is_empty {
                "*".to_string()
            } else {
                format!("*{}*", text)
            }
        }

        SelectionAction::Hyperlink => {
            if is_empty {
                "[](url)".to_string()
            } else {
                format!("[{}](url)", text)
            }
        }

        SelectionAction::Code => {
            if is_empty {
                "`".to_string()
            } else {
                format!("`{}`", text)
            }
        }

        SelectionAction::CodeBlock => {
            if is_empty {
                "```\n\n```".to_string()
            } else {
                format!("```\n{}\n```", text)
            }
        }

        SelectionAction::Image => {
            if is_empty {
                "![](image-url)".to_string()
            } else {
                format!("![{}](image-url)", text)
            }
        }

        SelectionAction::BulletedList => {
            if is_empty {
                "- ".to_string()
            } else {
                format_list(text, "- ")
            }
        }

        SelectionAction::NumberedList => {
            if is_empty {
                "1. ".to_string()
            } else {
                format_numbered_list(text)
            }
        }

        SelectionAction::CheckboxList => {
            if is_empty {
                "- [ ] ".to_string()
            } else {
                format_list(text, "- [ ] ")
            }
        }

        SelectionAction::Rule => "---".to_string(),
    }
}

/// Format text as a heading of the specified level
fn format_heading(text: &str, level: usize) -> String {
    let hashes = "#".repeat(level);

    if text.is_empty() {
        format!("{} ", hashes)
    } else {
        format!("{} {}", hashes, text.trim())
    }
}

/// Format text as a list with the given prefix
fn format_list(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| {
            if line.trim().is_empty() {
                prefix.to_string()
            } else {
                format!("{}{}", prefix, line.trim())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format text as a numbered list
fn format_numbered_list(text: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(i, line)| {
            if line.trim().is_empty() {
                format!("{}. ", i + 1)
            } else {
                format!("{}. {}", i + 1, line.trim())
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
