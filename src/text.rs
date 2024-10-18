use std::borrow::Cow;

use flecs_ecs::macros::Component;
use parley::style::{FontStack, StyleProperty};
use parley::{FontContext, Layout, LayoutContext};
use vello::kurbo::Affine;
use vello::peniko::{Color, Fill};
use vello::Scene;

// Singleton that handles writing text to scenes
#[derive(Component)]
pub struct TextWriter {
    font_cx: FontContext,
    layout_cx: LayoutContext<Color>,
}

impl TextWriter {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            font_cx: FontContext::default(),
            layout_cx: LayoutContext::new(),
        }
    }

    pub fn add(
        &mut self,
        scene: &mut Scene,
        transform: Affine,
        color: Color,
        size: f32,
        text: &str,
    ) {
        let mut builder = self.layout_cx.ranged_builder(&mut self.font_cx, text, 1.0);

        builder.push_default(StyleProperty::Brush(color));
        let font_stack = FontStack::Source(Cow::Owned("system-ui".to_string()));
        builder.push_default(StyleProperty::FontStack(font_stack));
        builder.push_default(StyleProperty::FontSize(size));

        let mut layout: Layout<Color> = builder.build(text);
        layout.break_all_lines(None);

        for line in layout.lines() {
            for positioned_layout in line.items() {
                match positioned_layout {
                    parley::PositionedLayoutItem::GlyphRun(glyph_run) => {
                        let run = glyph_run.run();
                        let style = glyph_run.style();
                        let font = run.font();

                        scene
                            .draw_glyphs(font)
                            .font_size(run.font_size())
                            .transform(transform)
                            .glyph_transform(None)
                            .brush(style.brush)
                            .hint(false)
                            .draw(
                                Fill::EvenOdd,
                                glyph_run.positioned_glyphs().map(|g| vello::Glyph {
                                    id: g.id as u32,
                                    x: g.x,
                                    y: g.y,
                                }),
                            );
                    }
                    parley::PositionedLayoutItem::InlineBox(_) => todo!(),
                }
            }
        }
    }
}
