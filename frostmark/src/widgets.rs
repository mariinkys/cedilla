//
// Original code by: Mrmayman <navneetkrishna22@gmail.com>
// https://github.com/Mrmayman/frostmark
//

use cosmic::iced::{Element, Font, Length, advanced, widget};

pub fn link_text<'a, M: 'a, F>(
    e: widget::text::Span<'a, M, Font>,
    url: String,
    msg: Option<&F>,
) -> widget::text::Span<'a, M, Font>
where
    F: Fn(String) -> M,
{
    e.link_maybe(msg.map(|n| n(url)))
}

pub fn underline<'a, M: 'a, T: widget::rule::Catalog + 'a, R: advanced::Renderer + 'a>(
    e: impl Into<Element<'a, M, T, R>>,
) -> widget::Stack<'a, M, T, R> {
    widget::stack!(
        widget::column![e.into()],
        widget::column![
            widget::space().height(Length::Fill),
            widget::rule::horizontal(1),
            widget::space().height(1),
        ]
    )
}
