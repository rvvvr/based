use vello::{SceneBuilder, kurbo::{Affine, Shape, Rect, Stroke, Point}, peniko::{BrushRef, Font, Color, Gradient, ColorStop, Blob}, glyph::Glyph};

use crate::{context::Viewport, dom::{Node, Element}, parser::css::{CSSValue, properties::Colour, Numeric}};

#[derive(Debug, Default)]
pub struct PageRenderer {}

impl PageRenderer {
    pub fn render(&mut self, viewport: Viewport, nodes: &Vec<Node>, builder: &mut SceneBuilder, last_width: f64) {
        for node in nodes {
            match node {
                Node::Element(el) => {
                    let color = if let CSSValue::Value(c) = el.css.background_color {
                        c.real
                    } else {
                        Colour::default().real
                    };
                    let shmop = el.layout_info.expand(el.layout_info.padding);
                    builder.fill(vello::peniko::Fill::NonZero, Affine::IDENTITY, BrushRef::Solid(Color::rgb8(color.red, color.green, color.blue)), Some(Affine::IDENTITY), &Rect::new(shmop.x, shmop.y, shmop.x + shmop.width, shmop.y + shmop.height));
                    self.render(viewport, &el.children, builder, last_width);
                },
		Node::LaidoutText(text) => {
		    let font_blob = Blob::new(text.font.copy_font_data().unwrap());
		    let font = Font::new(font_blob, 0);

		    builder.draw_glyphs(&font)
        .font_size(text.font_size as f32)
			.draw(vello::peniko::Fill::NonZero,text.glyphs.iter().map(|v| Glyph {
			    id: v.glyph.id as u32,
			    x: v.x as f32,
			    y: v.y as f32,
			}));
		},
                _ => {},
            }
        }
    }
}
