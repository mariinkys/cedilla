use dissimilar::Chunk;

/// Holds the state of the File History
#[derive(Default)]
pub struct HistoryState {
    pub history_base: String,
    pub history_patches: Vec<Vec<dissimilar::Chunk<'static>>>,
    pub history_index: usize,
}

impl HistoryState {
    pub fn new_with_content(content: String) -> Self {
        Self {
            history_base: content,
            history_patches: Vec::new(),
            history_index: 0,
        }
    }
}

/// Compute a patch (list of chunks) from `old` → `new`
pub fn make_patch(old: &str, new: &str) -> Vec<Chunk<'static>> {
    dissimilar::diff(old, new)
        .into_iter()
        .map(|chunk| match chunk {
            Chunk::Equal(s) => Chunk::Equal(Box::leak(s.to_string().into_boxed_str())),
            Chunk::Insert(s) => Chunk::Insert(Box::leak(s.to_string().into_boxed_str())),
            Chunk::Delete(s) => Chunk::Delete(Box::leak(s.to_string().into_boxed_str())),
        })
        .collect()
}

/// Apply a forward patch to `base`, reconstructing the text at that snapshot.
pub fn apply_patch(base: &str, patches: &[Vec<Chunk<'static>>]) -> String {
    patches.iter().fold(base.to_string(), |current, patch| {
        apply_single(&current, patch)
    })
}

pub fn apply_single(text: &str, patch: &[Chunk<'static>]) -> String {
    let mut result = String::with_capacity(text.len());
    for chunk in patch {
        match chunk {
            Chunk::Equal(s) | Chunk::Insert(s) => result.push_str(s),
            Chunk::Delete(_) => {}
        }
    }
    result
}
