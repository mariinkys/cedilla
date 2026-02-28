//
// Original code by: Mrmayman <navneetkrishna22@gmail.com>
// https://github.com/Mrmayman/frostmark
// I've only adapted it to work with libcosmic
//

use cosmic::iced::{Length, widget};
use markup5ever_rcdom::{Node, NodeData};
use std::rc::Rc;

use crate::{
    MarkWidget,
    renderer::{ValidTheme, get_attr},
    structs::{ChildAlignment, ChildData, ChildDataFlags, RenderedSpan},
};

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T> {
    pub fn draw_table(&mut self, node: &Node, data: ChildData) -> RenderedSpan<'a, M, T> {
        let mut header_cells: Vec<RenderedSpan<'a, M, T>> = Vec::new();
        let mut column_alignments: Vec<Option<ChildAlignment>> = Vec::new();
        let mut body_rows: Vec<Vec<RenderedSpan<'a, M, T>>> = Vec::new();

        let children = node.children.borrow();
        for section in children.iter() {
            let NodeData::Element { name, .. } = &section.data else {
                continue;
            };
            let section_name = name.local.to_string();

            let rows = section.children.borrow();
            for row in rows.iter() {
                let NodeData::Element { name, .. } = &row.data else {
                    continue;
                };
                if name.local.to_string() != "tr" {
                    continue;
                }

                let row_children = row.children.borrow();
                let cells: Vec<_> = row_children
                    .iter()
                    .filter(|cell| {
                        matches!(
                            &cell.data,
                            NodeData::Element { name, .. }
                                if matches!(name.local.to_string().as_str(), "th" | "td")
                        )
                    })
                    .collect();

                if section_name == "thead" || (header_cells.is_empty() && body_rows.is_empty()) {
                    // Header Cell
                    self.table_add_header_cell(
                        data,
                        &mut header_cells,
                        &mut column_alignments,
                        &cells,
                    );
                } else {
                    // Body Cell
                    body_rows.push(
                        cells
                            .iter()
                            .map(|cell| self.render_children(cell, data))
                            .collect(),
                    );
                }
            }
        }

        let body: cosmic::iced::Element<'a, M, T> = widget::column(
            body_rows
                .into_iter()
                .map(|row| draw_row(row, &column_alignments).into()),
        )
        .spacing(2)
        .into();

        widget::column![
            draw_row(header_cells, &column_alignments),
            widget::rule::horizontal(1),
            body,
        ]
        .spacing(4)
        .into()
    }

    fn table_add_header_cell(
        &mut self,
        data: ChildData,
        header_cells: &mut Vec<RenderedSpan<'a, M, T>>,
        column_alignments: &mut Vec<Option<ChildAlignment>>,
        cells: &[&Rc<Node>],
    ) {
        *column_alignments = cells
            .iter()
            .map(|cell| {
                let NodeData::Element { attrs, .. } = &cell.data else {
                    return None;
                };
                let attrs = attrs.borrow();
                match get_attr(&attrs, "align") {
                    Some("right") => Some(ChildAlignment::Right),
                    Some("center" | "centre") => Some(ChildAlignment::Center),
                    _ => None,
                }
            })
            .collect();

        *header_cells = cells
            .iter()
            .map(|cell| self.render_children(cell, data.insert(ChildDataFlags::BOLD)))
            .collect();
    }

    // fn draw_table(&mut self, node: &Node, data: ChildData) -> RenderedSpan<'a, M, T> {
    //     let mut header_cells: Vec<RenderedSpan<'a, M, T>> = Vec::new();
    //     let mut column_alignments: Vec<Option<ChildAlignment>> = Vec::new();
    //     let mut body_rows: Vec<Vec<RenderedSpan<'a, M, T>>> = Vec::new();

    //     let children = node.children.borrow();
    //     for section in children.iter() {
    //         let NodeData::Element { name, .. } = &section.data else {
    //             continue;
    //         };
    //         let section_name = name.local.to_string();

    //         let rows = section.children.borrow();
    //         for row in rows.iter() {
    //             let NodeData::Element { name, .. } = &row.data else {
    //                 continue;
    //             };
    //             if name.local.to_string() != "tr" {
    //                 continue;
    //             }

    //             let row_children = row.children.borrow();
    //             let cells: Vec<_> = row_children
    //                 .iter()
    //                 .filter(|cell| {
    //                     matches!(
    //                         &cell.data,
    //                         NodeData::Element { name, .. }
    //                             if matches!(name.local.to_string().as_str(), "th" | "td")
    //                     )
    //                 })
    //                 .collect();

    //             if section_name == "thead" || (header_cells.is_empty() && body_rows.is_empty()) {
    //                 column_alignments = cells
    //                     .iter()
    //                     .map(|cell| {
    //                         if let NodeData::Element { attrs, .. } = &cell.data {
    //                             let attrs = attrs.borrow();
    //                             match get_attr(&attrs, "align") {
    //                                 Some("right") => Some(ChildAlignment::Right),
    //                                 Some("center") | Some("centre") => Some(ChildAlignment::Center),
    //                                 _ => None,
    //                             }
    //                         } else {
    //                             None
    //                         }
    //                     })
    //                     .collect();

    //                 header_cells = cells
    //                     .iter()
    //                     .map(|cell| self.render_children(cell, data.insert(ChildDataFlags::BOLD)))
    //                     .collect();
    //             } else {
    //                 body_rows.push(
    //                     cells
    //                         .iter()
    //                         .map(|cell| self.render_children(cell, data))
    //                         .collect(),
    //                 );
    //             }
    //         }
    //     }

    //     let num_columns = header_cells.len();
    //     if num_columns == 0 {
    //         return RenderedSpan::None;
    //     }

    //     #[allow(clippy::type_complexity)]
    //     let mut per_column: Vec<
    //         Vec<std::cell::Cell<Option<cosmic::iced::Element<'a, M, T>>>>,
    //     > = (0..num_columns).map(|_| Vec::new()).collect();

    //     for row in body_rows {
    //         for (col_i, cell) in row.into_iter().enumerate() {
    //             if col_i < num_columns {
    //                 per_column[col_i].push(std::cell::Cell::new(Some(RenderedSpan::render(cell))));
    //             }
    //         }
    //     }

    //     let rendered_headers: Vec<cosmic::iced::Element<'a, M, T>> =
    //         header_cells.into_iter().map(RenderedSpan::render).collect();

    //     let row_count = per_column.first().map(|c| c.len()).unwrap_or(0);

    //     let columns = rendered_headers
    //         .into_iter()
    //         .zip(per_column)
    //         .enumerate()
    //         .map(|(col_i, (header, col_cells))| {
    //             let align_x = match column_alignments.get(col_i).copied().flatten() {
    //                 Some(ChildAlignment::Right) => cosmic::iced::alignment::Horizontal::Right,
    //                 Some(ChildAlignment::Center) => cosmic::iced::alignment::Horizontal::Center,
    //                 _ => cosmic::iced::alignment::Horizontal::Left,
    //             };
    //             cosmic::iced_widget::table::column(header, move |row_i: usize| {
    //                 col_cells[row_i]
    //                     .take()
    //                     .unwrap_or_else(|| widget::Space::new().into())
    //             })
    //             .align_x(align_x)
    //         });

    //     let table = cosmic::iced_widget::table::table(columns, 0..row_count);

    //     widget::scrollable(table)
    //         .direction(Direction::Horizontal(Scrollbar::default()))
    //         .width(cosmic::iced::Length::Fill)
    //         .spacing(3.)
    //         .into()
    // }
}

fn draw_row<'a, M: Clone + 'static, T: ValidTheme + 'a>(
    cells: Vec<RenderedSpan<'a, M, T>>,
    column_alignments: &[Option<ChildAlignment>],
) -> widget::Row<'a, M, T> {
    widget::row(
        cells
            .into_iter()
            .enumerate()
            .map(|(i, cell)| make_cell(cell, column_alignments.get(i).copied().flatten()).into()),
    )
    .spacing(2)
}

fn make_cell<'a, M: Clone + 'static, T: ValidTheme + 'a>(
    content: RenderedSpan<'a, M, T>,
    align: Option<ChildAlignment>,
) -> widget::Column<'a, M, T> {
    let alignment: cosmic::iced::Alignment =
        align.map_or(cosmic::iced::Alignment::Start, ChildAlignment::into);

    widget::column![content.render()]
        .align_x(alignment)
        .padding(5)
        .width(Length::Fill)
}
