use vello::{SceneBuilder, kurbo::{Affine, Shape, Rect, Stroke, Point}, peniko::{BrushRef, Color, Gradient, ColorStop}};

use crate::{context::Viewport, dom::Node, parser::css::{CSSValue, properties::Colour, Numeric}, layout::LayoutNode};

#[derive(Debug, Default)]
pub struct PageRenderer {

}

impl PageRenderer {
    pub fn render(&self, viewport: Viewport, nodes: &Vec<LayoutNode>, builder: &mut SceneBuilder, last_width: f64) {
        for node in nodes.iter().rev() {
            if let Some(el) = &node.element {
                let color = if let CSSValue::Value(c) = el.css.background_color {
                    c.real
                } else {
                    Colour::default().real
                };
                builder.fill(vello::peniko::Fill::NonZero, Affine::IDENTITY, BrushRef::Solid(Color::rgb8(color.red, color.green, color.blue)), Some(Affine::IDENTITY), &Rect::new(0., 0., node.absolute_width as f64, node.absolute_height as f64));
            }
            self.render(viewport, &node.children, builder, last_width / 2.);
        }
    } 
}
