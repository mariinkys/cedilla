use cosmic::iced_widget::scrollable;

/// Maps `source` viewport's relative scroll to an absolute Y in `target` viewport's content.
pub fn proportional_y(source: scrollable::Viewport, target: scrollable::Viewport) -> f32 {
    let rel = source.relative_offset().y;
    let target_scrollable_height =
        (target.content_bounds().height - target.bounds().height).max(0.0);
    (rel * target_scrollable_height).max(0.0)
}

/// Converts a vertical scroll position into an [`AbsoluteOffset`]
pub fn abs(y: f32) -> scrollable::AbsoluteOffset<Option<f32>> {
    scrollable::AbsoluteOffset {
        x: Some(0.0),
        y: Some(y),
    }
}
