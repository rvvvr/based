use vello::{SceneBuilder, kurbo::{Affine, Shape, Rect, Stroke, Point}, peniko::{BrushRef, Color, Gradient, ColorStop}};

use crate::{context::Viewport, dom::{Node, Element}, parser::css::{CSSValue, properties::Colour, Numeric}};

#[derive(Debug, Default)]
pub struct PageRenderer {
    top_offset: f64,
    left_offset: f64,
    right_offset: f64
}

impl PageRenderer {
    pub fn render(&mut self, viewport: Viewport, nodes: &Vec<Node>, builder: &mut SceneBuilder, last_width: f64) {
        self.top_offset = 0.;
        self.render_internal(viewport, nodes, builder, last_width);
    }
    fn render_internal(&mut self, viewport: Viewport, nodes: &Vec<Node>, builder: &mut SceneBuilder, last_width: f64) {
    } 
}
