use vello::{
    glyph::Glyph,
    kurbo::{Affine, Point, Rect, Shape, Stroke},
    peniko::{Blob, BrushRef, Color, ColorStop, Font, Gradient},
    SceneBuilder,
};

use crate::{
    context::Viewport,
    dom::{Element, Node},
    parser::css::{properties::Colour, CSSValue, Numeric},
};

#[derive(Debug, Default)]
pub struct PageRenderer {}

impl PageRenderer {
    pub fn render(
        &mut self,
        viewport: Viewport,
        nodes: &Vec<Node>,
        builder: &mut SceneBuilder,
        last_width: f64,
        render_info: RenderInfo,
    ) {
        for node in nodes {
            match node {
                Node::Element(el) => {
                    if el.layout_info.y + el.layout_info.height < render_info.scroll_y
                        || el.layout_info.y > render_info.scroll_y + viewport.height as f64
                    {
                        continue;
                    }
                    let color = if let CSSValue::Value(c) = el.css.background_color {
                        c.real
                    } else {
                        Colour::default().real
                    };
                    let shmop = el.layout_info.expand(el.layout_info.padding);
                    builder.fill(
                        vello::peniko::Fill::NonZero,
                        Affine::IDENTITY,
                        BrushRef::Solid(Color::rgb8(color.red, color.green, color.blue)),
                        Some(Affine::IDENTITY),
                        &Rect::new(
                            shmop.x,
                            shmop.y - render_info.scroll_y,
                            shmop.x + shmop.width,
                            shmop.y - render_info.scroll_y + shmop.height,
                        ),
                    );
                    self.render(viewport, &el.children, builder, last_width, render_info);
                }
                Node::LaidoutText(text) => {
                    let font_blob = Blob::new(text.font.copy_font_data().unwrap());
                    let font = Font::new(font_blob, 0);
                    let colour = text.colour.real;
                    let mut text_builder = builder.draw_glyphs(&font);
                    let mut text_builder = if text.axes.is_some() {
                        text_builder.normalized_coords(text.axes.as_ref().unwrap().as_slice())
                    } else {
                        text_builder
                    };

                    text_builder
                        .font_size(text.font_size as f32)
                        .brush(BrushRef::Solid(Color::rgba8(
                            colour.red,
                            colour.green,
                            colour.blue,
                            colour.alpha,
                        )))
                        .draw(
                            vello::peniko::Fill::NonZero,
                            text.glyphs.iter().map(|v| Glyph {
                                id: v.glyph.id as u32,
                                x: v.x as f32,
                                y: (v.y - render_info.scroll_y) as f32,
                            }),
                        );
                }
                _ => {}
            }
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct RenderInfo {
    pub scroll_y: f64,
    //scroll_x: f64,
}
