use std::{cell::RefCell, collections::HashMap};

use cosmic::{
    iced::{Alignment, Length, widget::image::Handle},
    widget,
};
use merman::diagram_theme::{DiagramThemeCompiler, ThemeAppearance, ThemeRecipeV1};
use merman::svg::{CssOverridePolicy, SvgOutputPolicy, SvgPipelinePreset, export::RasterOptions};
use merman::{OperationControl, PngRequest, RenderOutput, RenderRequest, Renderer, SvgRequest};

use crate::{MarkWidget, renderer::ValidTheme, structs::RenderedSpan};

const SCALE: f32 = 2.0;
/// Built-in merman design with paired Light/Dark appearances ("editor", "gruvbox" or "ayu").
const THEME_DESIGN: &str = "editor";
type Rendered = (Handle, f32);

thread_local! {
    // Survives state rebuilds. None = failed, don't retry every frame.
    static CACHE: RefCell<HashMap<(bool, String), Option<Rendered>>> = RefCell::default();
    // Last successful render per block position, shown while the source is invalid.
    static LAST_GOOD: RefCell<HashMap<usize, Rendered>> = RefCell::default();
}

impl<'a, M: Clone + 'static, T: ValidTheme + 'a> MarkWidget<'a, M, T> {
    pub fn draw_mermaid(&mut self, source: &str) -> RenderedSpan<'a, M, T> {
        let id = self.current_mermaid_id;
        self.current_mermaid_id += 1;

        let source = source.trim();
        let is_dark = cosmic::theme::active().cosmic().is_dark;
        let key = (is_dark, source.to_owned());

        let entry = CACHE
            .with_borrow(|c| c.get(&key).cloned())
            .unwrap_or_else(|| {
                let r = render_mermaid(source, is_dark).ok();
                CACHE.with_borrow_mut(|c| {
                    if c.len() > 64 {
                        c.clear(); // editing a diagram creates one key per keystroke
                    }
                    c.insert(key, r.clone());
                });
                r
            });

        let shown = match entry {
            Some(r) => {
                LAST_GOOD.with_borrow_mut(|m| m.insert(id, r.clone()));
                Some(r)
            }
            None => LAST_GOOD.with_borrow(|m| m.get(&id).cloned()),
        };

        match shown {
            Some((handle, width)) => {
                cosmic::iced::widget::column![widget::image(handle).width(Length::Fixed(width))]
                    .width(Length::Fill)
                    .align_x(Alignment::Center)
                    .into()
            }
            None => self.codeblock(source.to_string(), self.text_size, false),
        }
    }
}

pub fn render_mermaid_png(
    source: &str,
    is_dark: bool,
    scale: f32,
) -> Result<(Vec<u8>, f32), Box<dyn std::error::Error>> {
    let appearance = if is_dark {
        ThemeAppearance::Dark
    } else {
        ThemeAppearance::Light
    };

    let compiler = DiagramThemeCompiler::new();
    let mut recipe = compiler.export_preset_document(THEME_DESIGN)?;
    // Drop the preset's painted canvas so the diagram sits on the app background.
    if let ThemeRecipeV1::Design { design } = &mut recipe {
        design.canvas = None;
        for overrides in design.appearances.iter_mut().flat_map(|a| a.values_mut()) {
            overrides.canvas = None;
        }
    }
    let theme = compiler.compile_document(recipe, Some(appearance))?;

    let policy = SvgOutputPolicy {
        preset: SvgPipelinePreset::ResvgSafe,
        css_override_policy: CssOverridePolicy::StripExistingImportant,
        root_background_color: Some("transparent".to_string()),
        ..SvgOutputPolicy::default()
    };

    let output = Renderer::new().render(
        RenderRequest::png(
            source,
            OperationControl::new(),
            PngRequest {
                svg: SvgRequest {
                    pipeline: Some(policy.pipeline()),
                    ..Default::default()
                },
                options: RasterOptions::default().with_scale(scale),
            },
        )
        .with_theme(theme),
    )?;

    let RenderOutput::Png(Some(png)) = output else {
        return Err("no mermaid diagram detected".into());
    };

    let plan = png.plan();
    let width = plan.width_px as f32 / plan.effective_scale as f32;
    Ok((png.into_bytes(), width))
}

fn render_mermaid(source: &str, is_dark: bool) -> Result<Rendered, Box<dyn std::error::Error>> {
    let (bytes, width) = render_mermaid_png(source, is_dark, SCALE)?;

    let rgba =
        ::image::load_from_memory_with_format(&bytes, ::image::ImageFormat::Png)?.into_rgba8();
    let (w, h) = rgba.dimensions();
    Ok((Handle::from_rgba(w, h, rgba.into_raw()), width))
}
