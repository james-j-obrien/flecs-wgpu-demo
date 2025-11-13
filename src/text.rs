use std::borrow::Cow;

use flecs_ecs::macros::Component;
use parley::{FontContext, FontFamily, Layout, LayoutContext, PositionedLayoutItem};
use vello::kurbo::Affine;
use vello::peniko::{Color, Fill};
use vello::Glyph;
use vello::Scene;

// Define a simple brush type for parley that just stores RGBA bytes
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct SimpleBrush([u8; 4]);

// Singleton that handles writing text to scenes
#[derive(Component)]
#[flecs(traits(Singleton))]
pub struct TextWriter {
    font_cx: FontContext,
    layout_cx: LayoutContext<SimpleBrush>,
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
        // Convert vello Color to RGBA bytes for parley
        let rgba = SimpleBrush([
            (color.components[0] * 255.0) as u8,
            (color.components[1] * 255.0) as u8,
            (color.components[2] * 255.0) as u8,
            (color.components[3] * 255.0) as u8,
        ]);

        let mut builder = self
            .layout_cx
            .ranged_builder(&mut self.font_cx, text, 1.0, true);
        builder.push_default(FontFamily::parse("system-ui").unwrap());
        builder.push_default(parley::style::StyleProperty::FontSize(size));
        builder.push_default(parley::style::StyleProperty::Brush(rgba));

        let mut layout: Layout<SimpleBrush> = builder.build(text);
        layout.break_all_lines(Some(1000.0));
        layout.align(
            Some(1000.0),
            parley::Alignment::Start,
            parley::AlignmentOptions::default(),
        );

        for line in layout.lines() {
            for item in line.items() {
                if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                    let mut run_x = glyph_run.offset();
                    let run_y = glyph_run.baseline();
                    let run = glyph_run.run();
                    let style = glyph_run.style();

                    // Convert parley brush back to vello Color
                    // Components are u8 (0-255), need to normalize to f32 (0.0-1.0)
                    let brush_color = Color {
                        components: [
                            style.brush.0[0] as f32 / 255.0,
                            style.brush.0[1] as f32 / 255.0,
                            style.brush.0[2] as f32 / 255.0,
                            style.brush.0[3] as f32 / 255.0,
                        ],
                        cs: std::marker::PhantomData,
                    };

                    let glyphs = glyph_run.glyphs().map(|glyph| {
                        let glyph_x = run_x + glyph.x;
                        let glyph_y = run_y - glyph.y;
                        run_x += glyph.advance;
                        Glyph {
                            id: glyph.id as u32,
                            x: glyph_x,
                            y: glyph_y,
                        }
                    });

                    scene
                        .draw_glyphs(run.font())
                        .font_size(run.font_size())
                        .transform(transform)
                        .glyph_transform(None)
                        .brush(brush_color)
                        .hint(false)
                        .draw(Fill::NonZero, glyphs);
                }
            }
        }
    }
}
