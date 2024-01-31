use vello::{SceneBuilder, kurbo::{Affine, Shape, Rect}, peniko::{BrushRef, Color}};

use crate::{context::Viewport, dom::Node, parser::css::{CSSValue, properties::Colour, Numeric}};

#[derive(Debug, Default)]
pub struct PageRenderer {

}

impl PageRenderer {
    pub fn render(&self, viewport: Viewport, nodes: &Vec<Node>, builder: &mut SceneBuilder) {
        for node in nodes {
            if let Node::Element(el) = node {
                let color = if let CSSValue::Value(color) = &el.css.background_color {
                    color.real
                } else {
                    Colour::BLACK.real
                };
                let width = if let CSSValue::Value(width) = &el.css.width {
                    match width.value.unwrap() {
                        Numeric::Integer(n) => {
                            n as f64
                        },
                        Numeric::Number(n) => {
                            n as f64
                        }
                    }
                } else {
                    0.
                };
                let height = if let CSSValue::Value(height) = &el.css.height {
                    match height.value.unwrap() {
                        Numeric::Integer(n) => {
                            n as f64
                        },
                        Numeric::Number(n) => {
                            n as f64
                        }
                    }
                } else {
                    0.
                };
                builder.fill(vello::peniko::Fill::EvenOdd, Affine::IDENTITY, BrushRef::Solid(Color::rgba8(color.red, color.green, color.blue, color.alpha)), None, &Rect::new(0., 0., width, height));
                self.render(viewport, &el.children, builder);
            }
        }
    }
}
