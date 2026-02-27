use cosmic::iced::widget;
use markup5ever_rcdom::{Node, NodeData};

use crate::{
    MarkWidget, RubyMode,
    renderer::{ValidTheme, is_node_useless},
    structs::{ChildData, Emp, RenderedSpan},
};

struct RubyUnit<'a, M, T> {
    base: RenderedSpan<'a, M, T>,
    annotations: Vec<RenderedSpan<'a, M, T>>,
}

impl<M, T> Default for RubyUnit<'_, M, T> {
    fn default() -> Self {
        Self {
            base: RenderedSpan::None,
            annotations: Vec::new(),
        }
    }
}

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T> {
    pub(crate) fn draw_ruby(&mut self, node: &Node, data: ChildData) -> RenderedSpan<'a, M, T> {
        let units = self.ruby_collect_units(node, data);

        self.draw_ruby_units(units)
    }

    fn draw_ruby_units(&mut self, units: Vec<RubyUnit<'a, M, T>>) -> RenderedSpan<'a, M, T> {
        match self.ruby_mode {
            RubyMode::Ignore => {
                // only base content
                units
                    .into_iter()
                    .fold(RenderedSpan::None, |acc, u| acc + u.base)
            }

            RubyMode::Fallback => {
                // inline concat: base + annotations
                units.into_iter().fold(RenderedSpan::None, |acc, u| {
                    let ann = u
                        .annotations
                        .into_iter()
                        .fold(RenderedSpan::None, |a, b| a + b);
                    acc + u.base + ann
                })
            }

            RubyMode::Full => {
                // each unit is annotation above base
                units.into_iter().fold(RenderedSpan::None, |acc, u| {
                    let ann_block = u
                        .annotations
                        .into_iter()
                        .fold(RenderedSpan::None, |a, b| a + b);

                    let unit = RenderedSpan::Elem(
                        widget::column![ann_block.render(), u.base.render()]
                            .align_x(cosmic::iced::Alignment::Center)
                            .into(),
                        Emp::NonEmpty,
                    );

                    acc + unit
                })
            }
        }
    }

    fn ruby_collect_units(&mut self, node: &Node, data: ChildData) -> Vec<RubyUnit<'a, M, T>> {
        let mut units: Vec<RubyUnit<'a, M, T>> = Vec::new();
        let mut current = RubyUnit::default();

        for child in node.children.borrow().iter() {
            if is_node_useless(child) {
                continue;
            }

            match &child.data {
                NodeData::Element { name, .. } if &*name.local == "rb" => {
                    // flush previous
                    if !matches!(current.base, RenderedSpan::None) {
                        units.push(current);
                        current = RubyUnit::default();
                    }

                    current.base = self.render_children(child, data);
                }
                NodeData::Element { name, .. } if &*name.local == "rt" => {
                    current.annotations.push(self.traverse_node(child, data));
                }
                NodeData::Element { name, .. } if &*name.local == "rp" => {}

                _ => {
                    // implicit base
                    if !matches!(current.base, RenderedSpan::None) {
                        units.push(current);
                        current = RubyUnit::default();
                    }

                    current.base = self.traverse_node(child, data);
                }
            }
        }

        // flush last unit
        if !matches!(current.base, RenderedSpan::None) {
            units.push(current);
        }
        units
    }
}
