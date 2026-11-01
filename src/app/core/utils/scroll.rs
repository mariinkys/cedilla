use cosmic::iced::widget::scrollable::{self, Viewport};

/// Maps `source` viewport's relative scroll to an absolute Y in `target` viewport's content.
pub fn proportional_y(source: Viewport, target: Viewport) -> f32 {
    let rel = source.relative_offset().y;
    if !rel.is_finite() {
        return 0.0;
    }
    (rel.clamp(0.0, 1.0) * max_y(target)).max(0.0)
}

/// Largest vertical offset the scrollable can have.
pub fn max_y(viewport: Viewport) -> f32 {
    (viewport.content_bounds().height - viewport.bounds().height).max(0.0)
}

/// Converts a vertical scroll position into an [`AbsoluteOffset`]
pub fn abs(y: f32) -> scrollable::AbsoluteOffset<Option<f32>> {
    scrollable::AbsoluteOffset {
        x: Some(0.0),
        y: Some(y),
    }
}

/// What caused an `on_scroll` notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollOrigin {
    /// The offset didn't move: the content or the size changed (typing, images loading,
    /// window resize...). Must never be synced, or the other pane jumps around.
    Layout,
    /// The scroll we requested with `scroll_to`.
    Programmatic,
    /// Anything else moved the offset: the user (wheel, scrollbar, keyboard...).
    User,
}

/// Works out what caused a notification and consumes the pending expectation if it matched.
///
/// Iced only notifies when the viewport actually changes, so a `scroll_to` can produce no
/// notification at all. That's why this compares offsets instead of counting pending scrolls:
/// an expectation that never arrives is simply dropped on the next real scroll.
pub fn classify(last_y: Option<f32>, new_y: f32, expected: &mut Option<f32>) -> ScrollOrigin {
    const TOLERANCE: f32 = 1.0;

    let matches_expected = expected.is_some_and(|y| (y - new_y).abs() <= TOLERANCE);
    let offset_moved = last_y.is_some_and(|last_y| (last_y - new_y).abs() > 0.5);

    if !offset_moved {
        // a requested scroll to where we already were (e.g. back to the top on file open)
        if matches_expected {
            *expected = None;
        }
        return ScrollOrigin::Layout;
    }

    // the offset moved, so any pending expectation gets settled now
    if expected.take().is_some() && matches_expected {
        ScrollOrigin::Programmatic
    } else {
        ScrollOrigin::User
    }
}
