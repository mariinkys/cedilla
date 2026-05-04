use cosmic::{
    iced::{Alignment, Length},
    widget,
};

use crate::{
    MarkWidget,
    renderer::ValidTheme,
    structs::{RenderedSpan, TypstResult},
    typst_world::MinimalWorld,
};

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T> {
    pub fn draw_typst(&self, source: &str) -> RenderedSpan<'a, M, T> {
        if let Some(handle) = self.state.typst_cache.borrow().get(source) {
            return cosmic::iced::widget::column![
                widget::image(handle.clone()).width(Length::Shrink)
            ]
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .into();
        }

        let text_color = self.style.and_then(|s| s.text_color).unwrap_or_else(|| {
            let c = cosmic::theme::active().cosmic().on_bg_color();
            cosmic::iced::Color::from_rgb(c.red, c.green, c.blue)
        });

        let color_str = format!(
            "rgb({}, {}, {})",
            (text_color.r * 255.0) as u8,
            (text_color.g * 255.0) as u8,
            (text_color.b * 255.0) as u8,
        );
        let typst_size = self.text_size * 0.6;

        let wrapped = format!(
            "#set page(width: auto, height: auto, margin: 4pt, fill: none)\n\
                 #set text(fill: {}, size: {}pt)\n\
                 {}",
            color_str,
            &typst_size,
            source.trim()
        );

        match render_typst(&wrapped, 2.0) {
            Ok(r) => {
                self.state
                    .typst_cache
                    .borrow_mut()
                    .insert(source.trim().to_owned(), r.handle.clone());

                cosmic::iced::widget::column![widget::image(r.handle).width(Length::Shrink)]
                    .width(Length::Fill)
                    .align_x(Alignment::Center)
                    .into()
            }
            Err(_) => self.codeblock(source.to_string(), self.text_size, false),
        }
    }
}

pub fn render_typst(source: &str, pixel_per_pt: f32) -> Result<TypstResult, ()> {
    let world = MinimalWorld::new(source);
    let document: typst::layout::PagedDocument = typst::compile(&world).output.map_err(|_| ())?;
    let page = &document.pages[0];
    let pixmap = typst_render::render(page, pixel_per_pt);
    let width = pixmap.width();
    let height = pixmap.height();
    let rgba = unpremultiply(pixmap.take());

    Ok(TypstResult {
        handle: cosmic::iced::widget::image::Handle::from_rgba(width, height, rgba),
    })
}

fn unpremultiply(data: Vec<u8>) -> Vec<u8> {
    data.chunks_exact(4)
        .flat_map(|px| {
            let [r, g, b, a] = [px[0], px[1], px[2], px[3]];
            if a == 0 {
                return [0u8, 0, 0, 0];
            }
            let a_f = a as f32 / 255.0;
            [
                (r as f32 / a_f).round() as u8,
                (g as f32 / a_f).round() as u8,
                (b as f32 / a_f).round() as u8,
                a,
            ]
        })
        .collect()
}
