#![allow(clippy::collapsible_if)]

use cosmic::Task;
use std::sync::Arc;

use crate::app::{
    AppModel, Message, State,
    widgets::{markdown, text_editor},
};

/// Actions that can be performed on the current text selection
#[derive(Debug, Clone)]
pub enum SelectionAction {
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    Bold,
    Italic,
    Hyperlink,
    Code,
    CodeBlock,
    Image,
    BulletedList,
    NumberedList,
    CheckboxList,
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

        let selected_text = editor_content.selection();

        // If no selection, get the current line
        let (text, is_line) = match selected_text {
            Some(text) if !text.is_empty() => (text, false),
            _ => {
                let (line_index, _) = editor_content.cursor_position();
                match editor_content.line(line_index) {
                    Some(line) => (line.to_string(), true),
                    None => (String::new(), true),
                }
            }
        };

        // format the selected text
        let formatted = format_selected_text(&text, &action);

        if is_line {
            editor_content.perform(text_editor::Action::Move(text_editor::Motion::Home));
            editor_content.perform(text_editor::Action::Select(text_editor::Motion::End));
        }

        editor_content.perform(text_editor::Action::Edit(text_editor::Edit::Paste(
            Arc::new(formatted),
        )));

        *is_dirty = true;
        *items = markdown::parse(editor_content.text().as_ref()).collect();

        Task::none()
    }
}

/// Format the selected text based on the action
fn format_selected_text(text: &str, action: &SelectionAction) -> String {
    match action {
        SelectionAction::Heading1 => toggle_heading(text, 1),
        SelectionAction::Heading2 => toggle_heading(text, 2),
        SelectionAction::Heading3 => toggle_heading(text, 3),
        SelectionAction::Heading4 => toggle_heading(text, 4),
        SelectionAction::Heading5 => toggle_heading(text, 5),
        SelectionAction::Heading6 => toggle_heading(text, 6),
        SelectionAction::Bold => toggle_inline(text, "**"),
        SelectionAction::Italic => toggle_inline(text, "*"),
        SelectionAction::Hyperlink => toggle_hyperlink(text),
        SelectionAction::Code => toggle_inline(text, "`"),
        SelectionAction::CodeBlock => toggle_code_block(text),
        SelectionAction::Image => toggle_image(text),
        SelectionAction::BulletedList => toggle_list(text, ListKind::Bulleted),
        SelectionAction::NumberedList => toggle_list(text, ListKind::Numbered),
        SelectionAction::CheckboxList => toggle_list(text, ListKind::Checkbox),
        SelectionAction::Rule => toggle_rule(text),
    }
}

// ─── Headings ────────────────────────────────────────────────────────────────

fn toggle_heading(text: &str, level: usize) -> String {
    let hashes = "#".repeat(level);

    if let Some(stripped) = text.trim().strip_prefix(hashes.as_str()) {
        if stripped.starts_with(' ') || stripped.is_empty() {
            return stripped.trim().to_string();
        }
    }

    let stripped = text.trim().trim_start_matches('#').trim();

    if stripped.is_empty() {
        format!("{} ", hashes)
    } else {
        format!("{} {}", hashes, stripped)
    }
}

fn toggle_inline(text: &str, marker: &str) -> String {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return marker.to_string();
    }

    if marker == "*" {
        if trimmed.starts_with("**") && trimmed.ends_with("**") {
            return format!("*{}*", trimmed);
        }
    }

    if trimmed.starts_with(marker) && trimmed.ends_with(marker) && trimmed.len() > marker.len() * 2
    {
        trimmed[marker.len()..trimmed.len() - marker.len()].to_string()
    } else {
        format!("{}{}{}", marker, trimmed, marker)
    }
}

fn toggle_hyperlink(text: &str) -> String {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return "[](url)".to_string();
    }

    if trimmed.starts_with('[') && trimmed.contains("](") && trimmed.ends_with(')') {
        if let Some(display_end) = trimmed.find("](") {
            return trimmed[1..display_end].to_string();
        }
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return format!("[{}]({})", trimmed, trimmed);
    }

    format!("[{}](url)", trimmed)
}

fn toggle_code_block(text: &str) -> String {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return "```\n\n```".to_string();
    }

    if trimmed.starts_with("```") && trimmed.ends_with("```") && trimmed.len() > 6 {
        let inner = trimmed[3..trimmed.len() - 3].trim();

        let inner = if let Some(newline) = inner.find('\n') {
            let first_line = inner[..newline].trim();
            if first_line
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                inner[newline..].trim()
            } else {
                inner
            }
        } else {
            inner
        };
        return inner.to_string();
    }

    format!("```\n{}\n```", trimmed)
}

fn toggle_image(text: &str) -> String {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return "![alt text](image-url)".to_string();
    }

    if trimmed.starts_with("![") && trimmed.contains("](") && trimmed.ends_with(')') {
        if let Some(alt_end) = trimmed.find("](") {
            return trimmed[2..alt_end].to_string();
        }
    }

    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
        return format!("![alt text]({})", trimmed);
    }

    format!("![{}](image-url)", trimmed)
}

enum ListKind {
    Bulleted,
    Numbered,
    Checkbox,
}

fn toggle_list(text: &str, kind: ListKind) -> String {
    if text.trim().is_empty() {
        return match kind {
            ListKind::Bulleted => "- ".to_string(),
            ListKind::Numbered => "1. ".to_string(),
            ListKind::Checkbox => "- [ ] ".to_string(),
        };
    }

    let lines: Vec<&str> = text.lines().collect();

    let all_match = lines.iter().all(|line| {
        let t = line.trim();
        if t.is_empty() {
            return true;
        }
        match kind {
            ListKind::Bulleted => {
                (t.starts_with("- ") || t.starts_with("* ") || t.starts_with("+ "))
                    && !t.starts_with("- [ ]")
                    && !t.starts_with("- [x]")
                    && !t.starts_with("- [X]")
            }
            ListKind::Numbered => t.starts_with(|c: char| c.is_numeric()) && t.contains(". "),
            ListKind::Checkbox => {
                t.starts_with("- [ ]") || t.starts_with("- [x]") || t.starts_with("- [X]")
            }
        }
    });

    if all_match {
        lines
            .iter()
            .map(|line| {
                let t = line.trim();
                if t.is_empty() {
                    return String::new();
                }
                match kind {
                    ListKind::Bulleted => t
                        .trim_start_matches("- ")
                        .trim_start_matches("* ")
                        .trim_start_matches("+ ")
                        .to_string(),
                    ListKind::Numbered => {
                        if let Some(dot) = t.find(". ") {
                            t[dot + 2..].to_string()
                        } else {
                            t.to_string()
                        }
                    }
                    ListKind::Checkbox => t
                        .trim_start_matches("- [ ] ")
                        .trim_start_matches("- [x] ")
                        .trim_start_matches("- [X] ")
                        .to_string(),
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                let t = strip_any_list_marker(line.trim());
                if t.is_empty() {
                    return match kind {
                        ListKind::Bulleted => "- ".to_string(),
                        ListKind::Numbered => format!("{}. ", i + 1),
                        ListKind::Checkbox => "- [ ] ".to_string(),
                    };
                }
                match kind {
                    ListKind::Bulleted => format!("- {}", t),
                    ListKind::Numbered => format!("{}. {}", i + 1, t),
                    ListKind::Checkbox => format!("- [ ] {}", t),
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Strip any known list prefix from a line
fn strip_any_list_marker(text: &str) -> String {
    // Checkbox
    if let Some(s) = text
        .strip_prefix("- [ ] ")
        .or_else(|| text.strip_prefix("- [x] "))
        .or_else(|| text.strip_prefix("- [X] "))
    {
        return s.to_string();
    }

    // Bulleted
    if let Some(s) = text
        .strip_prefix("- ")
        .or_else(|| text.strip_prefix("* "))
        .or_else(|| text.strip_prefix("+ "))
    {
        return s.to_string();
    }
    // Numbered
    if let Some(dot) = text.find(". ") {
        let prefix = &text[..dot];
        if prefix.chars().all(|c| c.is_numeric()) {
            return text[dot + 2..].to_string();
        }
    }
    text.to_string()
}

fn toggle_rule(text: &str) -> String {
    let trimmed = text.trim();

    if trimmed == "---" || trimmed == "***" || trimmed == "___" {
        return String::new();
    }

    if trimmed.is_empty() {
        "---".to_string()
    } else {
        format!("{}\n\n---", trimmed)
    }
}
