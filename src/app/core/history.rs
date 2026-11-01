/// Holds the state of the File History.
///
/// Stores full snapshots: Markdown notes are small, so a copy per step (capped at
/// [`MAX_HISTORY`]) is cheaper than diffing the whole document on every keystroke.
#[derive(Default)]
pub struct HistoryState {
    /// Text before the oldest undo step
    pub history_base: String,
    /// `history_snapshots[i]` is the text after edit `i`
    pub history_snapshots: Vec<String>,
    /// How many edits are currently applied (0 = `history_base`)
    pub history_index: usize,
    pub cursor_before: Vec<(usize, usize)>, // cursor before edit i
    pub cursor_after: Vec<(usize, usize)>,  // cursor after edit i
}

pub const MAX_HISTORY: usize = 100;

impl HistoryState {
    pub fn new_with_content(content: String) -> Self {
        Self {
            history_base: content,
            ..Self::default()
        }
    }

    /// Text with the first `index` edits applied (0 = base).
    pub fn snapshot(&self, index: usize) -> &str {
        match index {
            0 => &self.history_base,
            i => &self.history_snapshots[i - 1],
        }
    }
}
