// SPDX-License-Identifier: GPL-3.0

use std::sync::Arc;

use cosmic::Task;

use crate::app::{AppModel, Message, State, widgets::text_editor};

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
    /// Convert selection to code
    Code,
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
            ..
        } = &mut self.state
        else {
            return Task::none();
        };

        let selection = editor_content.selection().unwrap_or_default();
        let formatted = format_selected_text(&selection, action);
        let formatted = formatted.trim_end_matches('\n').to_string();
        let formatted_len = formatted.chars().count();

        editor_content.perform(text_editor::Action::Edit(text_editor::Edit::Paste(
            Arc::new(formatted),
        )));

        for _ in 0..formatted_len {
            editor_content.perform(text_editor::Action::Move(text_editor::Motion::Left));
        }

        for _ in 0..formatted_len {
            editor_content.perform(text_editor::Action::Select(text_editor::Motion::Right));
        }

        *is_dirty = true;
        //*items = markdown::parse(editor_content.text().as_ref()).collect();

        Task::none()
    }
}

/// Format the selected text based on the action
fn format_selected_text(text: &str, action: SelectionAction) -> String {
    let is_empty = text.is_empty();

    match action {
        SelectionAction::Heading1 => toggle_heading(text, 1),
        SelectionAction::Heading2 => toggle_heading(text, 2),
        SelectionAction::Heading3 => toggle_heading(text, 3),
        SelectionAction::Heading4 => toggle_heading(text, 4),
        SelectionAction::Heading5 => toggle_heading(text, 5),
        SelectionAction::Heading6 => toggle_heading(text, 6),

        SelectionAction::Bold => {
            if is_empty {
                "****".to_string()
            } else if text.starts_with("**") && text.ends_with("**") && text.len() >= 4 {
                text[2..text.len() - 2].to_string()
            } else {
                format!("**{}**", text)
            }
        }

        SelectionAction::Italic => {
            if is_empty {
                "**".to_string()
            } else if is_italic(text) {
                text[1..text.len() - 1].to_string()
            } else {
                format!("*{}*", text)
            }
        }

        SelectionAction::Hyperlink => {
            if is_empty {
                "[](url)".to_string()
            } else if text.starts_with('[') && text.ends_with(')') {
                text.to_string()
            } else {
                format!("[{}](url)", text)
            }
        }

        SelectionAction::Code => {
            if is_empty {
                // cycle: nothing → inline → block → nothing
                "``".to_string()
            } else if text.starts_with("```") && text.ends_with("```") && text.len() > 6 {
                // code block → nothing (strip fences)
                let inner = &text[3..text.len() - 3];
                inner.trim_matches('\n').to_string()
            } else if text.starts_with('`') && text.ends_with('`') && text.len() >= 2 {
                // inline code → code block
                let inner = &text[1..text.len() - 1];
                format!("```\n{}\n```", inner)
            } else {
                // nothing → inline code
                format!("`{}`", text)
            }
        }

        SelectionAction::Image => {
            if is_empty {
                "![](image-url)".to_string()
            } else if text.starts_with("![") && text.ends_with(')') {
                text.to_string()
            } else {
                format!("![{}](image-url)", text)
            }
        }

        SelectionAction::BulletedList => {
            if is_empty {
                "- ".to_string()
            } else if all_lines_have_prefix(text, "- ") {
                remove_line_prefix(text, "- ")
            } else {
                format_list(text, "- ")
            }
        }

        SelectionAction::NumberedList => {
            if is_empty {
                "1. ".to_string()
            } else if all_lines_are_numbered(text) {
                remove_numbered_list(text)
            } else {
                format_numbered_list(text)
            }
        }

        SelectionAction::CheckboxList => {
            if is_empty {
                "- [ ] ".to_string()
            } else if all_lines_have_prefix(text, "- [ ] ") || all_lines_have_prefix(text, "- [x] ")
            {
                remove_line_prefix(remove_line_prefix(text, "- [ ] ").as_str(), "- [x] ")
            } else {
                format_list(text, "- [ ] ")
            }
        }

        SelectionAction::Rule => "---".to_string(),
    }
}

fn toggle_heading(text: &str, level: usize) -> String {
    let hashes = "#".repeat(level);

    if text.is_empty() {
        return format!("{} ", hashes);
    }

    let trimmed = text.trim();

    // if already this heading level
    let this_prefix = format!("{} ", hashes);
    if trimmed.starts_with(&this_prefix) {
        return trimmed[this_prefix.len()..].to_string();
    }

    // check if it's a different heading level, strip it and apply new one
    let without_existing = strip_heading(trimmed);
    format!("{} {}", hashes, without_existing.trim())
}

/// Strip any leading heading markers from text
fn strip_heading(text: &str) -> &str {
    let mut chars = text.chars().peekable();
    let mut count = 0;
    while chars.peek() == Some(&'#') {
        chars.next();
        count += 1;
    }
    if count > 0 && chars.peek() == Some(&' ') {
        &text[count + 1..]
    } else {
        text
    }
}

/// Returns true if text is italic but not bold
fn is_italic(text: &str) -> bool {
    text.starts_with('*') && text.ends_with('*') && text.len() >= 2 && !text.starts_with("**")
}

/// Returns true if every line starts with the given prefix
fn all_lines_have_prefix(text: &str, prefix: &str) -> bool {
    text.lines().all(|line| line.starts_with(prefix))
}

/// Remove a prefix from every line
fn remove_line_prefix(text: &str, prefix: &str) -> String {
    text.lines()
        .map(|line| line.strip_prefix(prefix).unwrap_or(line).to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Returns true if every line looks like a numbered list item
fn all_lines_are_numbered(text: &str) -> bool {
    text.lines().enumerate().all(|(i, line)| {
        let prefix = format!("{}. ", i + 1);
        line.starts_with(&prefix)
    })
}

/// Remove numbered list formatting from every line
fn remove_numbered_list(text: &str) -> String {
    text.lines()
        .enumerate()
        .map(|(i, line)| {
            let prefix = format!("{}. ", i + 1);
            if line.starts_with(&prefix) {
                line[prefix.len()..].to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

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
