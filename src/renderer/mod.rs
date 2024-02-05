use vello::{SceneBuilder, kurbo::{Affine, Shape, Rect, Stroke, Point}, peniko::{BrushRef, Color, Gradient, ColorStop}};

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
                _ => {},
            }
        }
    }
}
