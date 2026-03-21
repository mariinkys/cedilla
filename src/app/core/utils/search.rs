use grep::matcher::{Match, Matcher};
use grep::regex::RegexMatcherBuilder;
use grep::searcher::{Searcher, sinks::UTF8};

use crate::app::core::editor::EditorSearchState;
use cosmic::iced::core::text::editor::Cursor;

/// Actions related to the editor search feature
#[derive(Debug, Clone)]
pub enum SearchAction {
    ToggleSearch,
    UpdateSearchValue(String),
    ToggleRegex,
    NextResult,
    PrevResult,

    FocusSearchField,
}

#[derive(Debug, Clone)]
pub struct SearchMatch {
    pub line: usize,
    pub col_start: usize,
    pub col_end: usize,
}

impl From<&SearchMatch> for Cursor {
    fn from(value: &SearchMatch) -> Self {
        use cosmic::widget::text_editor::Position;

        Self {
            position: Position {
                line: value.line,
                column: value.col_end,
            },
            selection: Some(Position {
                line: value.line,
                column: value.col_start,
            }),
        }
    }
}

impl EditorSearchState {
    /// Recomputes all matches against `text`
    pub fn compute_matches(&mut self, text: &str) {
        self.matches.clear();
        self.regex_error = None;

        if self.search_value.is_empty() {
            self.current_match_index = None;
            return;
        }

        // fixed_strings(true) gives literal search; false gives full regex.
        let matcher = match RegexMatcherBuilder::new()
            .fixed_strings(!self.use_regex)
            .build(&self.search_value)
        {
            Ok(m) => m,
            Err(e) => {
                self.regex_error = Some(e.to_string());
                self.current_match_index = None;
                return;
            }
        };

        let mut searcher = Searcher::new();
        let mut new_matches: Vec<SearchMatch> = Vec::new();

        let _ = searcher.search_slice(
            &matcher,
            text.as_bytes(),
            UTF8(|line_number, line_text| {
                let line_idx = (line_number as usize).saturating_sub(1);

                let _ = matcher.find_iter(line_text.as_bytes(), |m: Match| {
                    new_matches.push(SearchMatch {
                        line: line_idx,
                        col_start: m.start(),
                        col_end: m.end(),
                    });

                    true // keep iterating
                });

                Ok(true) // keep searching further lines
            }),
        );

        self.matches = new_matches;
        self.current_match_index = if self.matches.is_empty() {
            None
        } else {
            Some(
                self.current_match_index
                    .map(|i| i.min(self.matches.len() - 1))
                    .unwrap_or(0),
            )
        };
    }

    pub fn next_match(&mut self) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        let next = self
            .current_match_index
            .map(|i| (i + 1) % self.matches.len())
            .unwrap_or(0);
        self.current_match_index = Some(next);
        self.matches.get(next)
    }

    pub fn prev_match(&mut self) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }
        let len = self.matches.len();
        let prev = self
            .current_match_index
            .map(|i| if i == 0 { len - 1 } else { i - 1 })
            .unwrap_or(0);
        self.current_match_index = Some(prev);
        self.matches.get(prev)
    }
}
